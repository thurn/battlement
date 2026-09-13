use battlement_native::ConnectView;

const MODULE_ID: &str = "battlement.diagnostics";

pub(crate) fn is_available(connect: ConnectView<'_>) -> bool {
  connect.modules().any(|module| module == MODULE_ID)
}
