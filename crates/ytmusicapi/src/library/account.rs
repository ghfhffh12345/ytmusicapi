use serde_json::Value;

use crate::{AccountInfo, Error};

use super::core::{optional_runs_text, required_runs_text, required_text};

pub(crate) fn parse_account_info_response(response: &Value) -> Result<AccountInfo, Error> {
    let header = response
        .pointer(
            "/actions/0/openPopupAction/popup/multiPageMenuRenderer/header/activeAccountHeaderRenderer",
        )
        .ok_or_else(|| {
            Error::Parse(
                "account menu response missing activeAccountHeaderRenderer".to_owned(),
            )
        })?;

    Ok(AccountInfo {
        account_name: required_runs_text(header, "/accountName/runs")?,
        channel_handle: optional_runs_text(header, "/channelHandle/runs"),
        account_photo_url: required_text(header, "/accountPhoto/thumbnails/0/url")?,
    })
}
