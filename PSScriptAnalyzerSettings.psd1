@{
    # 本仓库的 smoke / CI 脚本通过 Write-Host 把进度与诊断信息直接写入
    # GitHub Actions 日志；同时其中若干脚本的返回值会被 CI 解析为 JSON
    # summary。若为了满足 PSAvoidUsingWriteHost 而改成 Write-Output，消息会
    # 混入被解析的返回值并破坏 summary，因此这里显式排除该规则，而不是
    # 改写这些脚本的输出语义。
    # 其余规则保持启用；如出现新的告警，应先评估是否可修复，再考虑排除。
    ExcludeRules = @(
        'PSAvoidUsingWriteHost'
    )
}
