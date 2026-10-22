use std::{ffi::OsString, time::Duration};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

define_windows_service!(ffi_service_main, service_main);
pub fn dispatch() -> anyhow::Result<()> {
    service_dispatcher::start("Homeboard", ffi_service_main)?;
    Ok(())
}
fn service_main(_arguments: Vec<OsString>) {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let Ok(handle) = service_control_handler::register("Homeboard", move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = tx.send(true);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }) else {
        return;
    };
    let status = |state, code| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(code),
        checkpoint: if state == ServiceState::StartPending {
            1
        } else {
            0
        },
        wait_hint: Duration::from_secs(15),
        process_id: None,
    };
    let _ = handle.set_service_status(status(ServiceState::StartPending, 0));
    let result = (|| -> anyhow::Result<()> {
        let args: Vec<String> = std::env::args().collect();
        let dir = crate::data_dir(&args)?;
        let _guard = crate::logging(&dir)?;
        let result = (|| -> anyhow::Result<()> {
            let runtime = tokio::runtime::Runtime::new()?;
            let listener = crate::bind(&crate::read_server(&dir)?)?;
            crate::config::load(&dir)?;
            crate::auth::AuthFile::load(&dir)?;
            handle.set_service_status(status(ServiceState::Running, 0))?;
            runtime.block_on(crate::run(dir, rx, false, Some(listener)))
        })();
        if result.is_err() {
            tracing::error!("service_failed_check_port_config_and_permissions");
        }
        result
    })();
    let _ = handle.set_service_status(status(
        ServiceState::Stopped,
        if result.is_ok() { 0 } else { 1 },
    ));
}
