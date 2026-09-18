#Requires -Version 7
<#
.SYNOPSIS
    Summarizes a query stage profile log written through FLUX_QUERY_PROFILE_FILE.

.DESCRIPTION
    The launcher writes one tab-separated line per stage event when
    FLUX_QUERY_PROFILE_FILE points at a writable file:

        {unix_ms}  {event}  {detail}

    Events today: catalog-search ({ms} {resultCount} {query}), icon-request
    (queued={depth} {fileName}) and icon-extract ({ms} {hit} queued={remaining}).

    This script aggregates those lines so a single run answers the question the
    u-key lag report needs: is the time spent in catalog filtering or in shell
    icon extraction, and how deep does the icon queue get. It only reads the
    log; nothing is written back.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$ProfilePath,

    [int]$SlowThresholdMs = 50,

    [int]$TopSlowCount = 15,

    # 图标队列上限（同步通道容量 64 + pending 集合），超出即视为写入交错的坏行。
    [int]$MaxSaneQueueDepth = 512
)

if (-not (Test-Path $ProfilePath)) {
    throw "找不到 profile 日志：$ProfilePath。请确认已设置 FLUX_QUERY_PROFILE_FILE 且启动器已运行。"
}

$lines = @(Get-Content $ProfilePath | Where-Object { $_.Trim() -ne '' })
if ($lines.Count -eq 0) {
    throw "profile 日志为空：$ProfilePath。请先复现卡顿（例如连续键入 u）再分析。"
}

$catalog = [System.Collections.Generic.List[object]]::new()
$extract = [System.Collections.Generic.List[object]]::new()
$requestDepths = [System.Collections.Generic.List[int]]::new()
$extractQueued = [System.Collections.Generic.List[int]]::new()
$extractMisses = 0
$unknownEvents = [System.Collections.Generic.HashSet[string]]::new()
$skippedLines = 0

foreach ($line in $lines) {
    $parts = $line -split "`t", 3
    if ($parts.Count -lt 2 -or $parts[0] -notmatch '^[0-9]+\.[0-9]+$') {
        # 时间戳缺失或写入交错留下的残片：计为 skipped，不混入 unknownEvents。
        $skippedLines++
        continue
    }

    $eventName = $parts[1]
    $detail = if ($parts.Count -ge 3) { $parts[2] } else { '' }

    if ($eventName -notmatch '^[a-z][a-z0-9-]*$') {
        # 写入交错会把时间戳残片黏到事件名上（例如 "1789722894363icon-request."）。
        $skippedLines++
        continue
    }

    switch ($eventName) {
        'catalog-search' {
            $fields = $detail -split "`t"
            if ($fields.Count -ge 2) {
                $query = if ($fields.Count -ge 3) { $fields[2] } else { '' }
                $catalog.Add([pscustomobject]@{
                    Stage        = 'catalog-search'
                    Milliseconds = [double]($fields[0] -replace 'ms$', '')
                    ResultCount  = [int]$fields[1]
                    Query        = $query
                })
            }
        }
        'icon-extract' {
            $fields = $detail -split "`t"
            if ($fields.Count -ge 2) {
                $queued = 0
                if ($fields.Count -ge 3 -and $fields[2] -match 'queued=(\d+)') {
                    $depth = [int64]$Matches[1]
                    if ($depth -lt 0 -or $depth -gt $MaxSaneQueueDepth) {
                        # 写入交错可能把两个数字黏成一个超大值；该行整体不可信。
                        $skippedLines++
                        continue
                    }
                    $queued = [int]$depth
                }
                $hit = $fields[1] -eq 'true'
                if (-not $hit) {
                    $extractMisses++
                }
                $extractQueued.Add($queued)
                $extract.Add([pscustomobject]@{
                    Stage        = 'icon-extract'
                    Milliseconds = [double]($fields[0] -replace 'ms$', '')
                    Hit          = $hit
                    Detail       = if ($hit) { 'hit' } else { 'miss' }
                    QueuedAfter  = $queued
                })
            }
        }
        'icon-request' {
            if ($detail -match 'queued=(\d+)') {
                $depth = [int64]$Matches[1]
                if ($depth -lt 0 -or $depth -gt $MaxSaneQueueDepth) {
                    $skippedLines++
                    continue
                }
                $requestDepths.Add([int]$depth)
            }
        }
        default {
            [void]$unknownEvents.Add($eventName)
        }
    }
}

function Get-Percentile {
    param([double[]]$Values, [double]$Percentile)

    if ($Values.Count -eq 0) {
        return 0
    }
    $sorted = @($Values | Sort-Object)
    $index = [Math]::Min($sorted.Count - 1, [Math]::Ceiling($Percentile * $sorted.Count) - 1)
    return [Math]::Round($sorted[[Math]::Max(0, $index)], 2)
}

function Write-StageSummary {
    param([string]$Name, [System.Collections.Generic.List[object]]$Samples)

    if ($Samples.Count -eq 0) {
        Write-Host "$Name`: 无样本"
        return
    }
    $values = [double[]]@($Samples | ForEach-Object { $_.Milliseconds })
    $total = ($values | Measure-Object -Sum).Sum
    Write-Host ("{0}: count={1} total={2}ms avg={3}ms p50={4}ms p95={5}ms max={6}ms" -f `
        $Name, $Samples.Count, [Math]::Round($total, 2), [Math]::Round($total / $Samples.Count, 2), `
        (Get-Percentile $values 0.5), (Get-Percentile $values 0.95), (Get-Percentile $values 1.0))
}

Write-Host "profile 日志：$ProfilePath"
Write-Host "样本行数：$($lines.Count)"
Write-Host "skipped=$skippedLines（时间戳缺失或写入残片，未计入任何统计）"
Write-Host ''

Write-StageSummary -Name 'catalog-search' -Samples $catalog
Write-StageSummary -Name 'icon-extract' -Samples $extract

if ($extract.Count -gt 0) {
    $missRate = [Math]::Round(100.0 * $extractMisses / $extract.Count, 1)
    Write-Host "icon-extract 结论：命中 $($extract.Count - $extractMisses) / $($extract.Count)，miss 率 ${missRate}%"
}

if ($requestDepths.Count -gt 0) {
    $depthValues = [double[]]@($requestDepths)
    Write-Host ("icon-request 队列深度：count={0} max={1} p95={2}" -f `
        $requestDepths.Count, ($requestDepths | Measure-Object -Maximum).Maximum, (Get-Percentile $depthValues 0.95))
}
if ($extractQueued.Count -gt 0) {
    Write-Host "icon-extract 完成后剩余队列最大深度：$(($extractQueued | Measure-Object -Maximum).Maximum)"
}

if ($unknownEvents.Count -gt 0) {
    Write-Host "未知事件类型（脚本未覆盖）：$([string]::Join(', ', $unknownEvents))"
}

Write-Host ''
Write-Host "慢样本（≥ ${SlowThresholdMs}ms，最多 $TopSlowCount 条）"
$slow = @($catalog + $extract | Where-Object { $_.Milliseconds -ge $SlowThresholdMs } |
    Sort-Object Milliseconds -Descending | Select-Object -First $TopSlowCount)
if ($slow.Count -eq 0) {
    Write-Host '  无'
} else {
    $slow | ForEach-Object {
        $context = if ($_.Stage -eq 'catalog-search') {
            "query='$($_.Query)' results=$($_.ResultCount)"
        } else {
            "$($_.Detail) queued_after=$($_.QueuedAfter)"
        }
        Write-Host ("  {0}ms {1} {2}" -f $_.Milliseconds, $_.Stage, $context)
    }
}

# 瓶颈判定只基于可核对的数字：单条图标提取的最坏值、整体超阈值样本占比与目录搜索耗时。
$worstExtract = if ($extract.Count -gt 0) { ($extract | Measure-Object -Property Milliseconds -Maximum).Maximum } else { 0 }
$worstCatalog = if ($catalog.Count -gt 0) { ($catalog | Measure-Object -Property Milliseconds -Maximum).Maximum } else { 0 }
$slowRatio = if (($catalog.Count + $extract.Count) -gt 0) {
    [Math]::Round(100.0 * $slow.Count / ($catalog.Count + $extract.Count), 1)
} else {
    0
}
Write-Host ''
Write-Host "判定线索：catalog 最坏 ${worstCatalog}ms，icon-extract 最坏 ${worstExtract}ms，超阈值样本占比 ${slowRatio}%（阈值 ${SlowThresholdMs}ms）"