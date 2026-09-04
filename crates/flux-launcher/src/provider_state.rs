use crate::applications::{ApplicationResponse, ApplicationWorker};
use crate::everything::{EverythingResponse, EverythingWorker};
use crate::plugins::{
    FlowPluginWorker, NativePluginQueryResponse, NativePluginWorker, PluginQueryResponse,
};
use windui::prelude::Sender;

pub(crate) struct ProviderWorkers {
    pub(crate) applications: ApplicationWorker,
    pub(crate) everything: EverythingWorker,
    pub(crate) plugins: FlowPluginWorker,
    pub(crate) native_plugins: NativePluginWorker,
}

impl ProviderWorkers {
    pub(crate) fn new(
        application_sender: Sender<ApplicationResponse>,
        everything_sender: Sender<EverythingResponse>,
        plugin_sender: Sender<PluginQueryResponse>,
        native_plugin_sender: Sender<NativePluginQueryResponse>,
    ) -> Self {
        Self {
            applications: ApplicationWorker::spawn(application_sender),
            everything: EverythingWorker::spawn(everything_sender),
            plugins: FlowPluginWorker::spawn(plugin_sender),
            native_plugins: NativePluginWorker::spawn(native_plugin_sender),
        }
    }
}
