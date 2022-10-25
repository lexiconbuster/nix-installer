use std::path::PathBuf;

use crate::{
    actions::{Action, ActionDescription, ActionError, Actionable},
    BuiltinPlanner, HarmonicError,
};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct InstallPlan {
    pub(crate) actions: Vec<Action>,

    pub(crate) planner: BuiltinPlanner,
}

impl InstallPlan {
    #[tracing::instrument(skip_all)]
    pub fn describe_execute(&self, explain: bool) -> String {
        let Self { planner, actions } = self;
        format!(
            "\
            This Nix install is for:\n\
              Operating System: {os_type}\n\
              Init system: {init_type}\n\
              Nix channels: {nix_channels}\n\
            \n\
            Created by planner: {planner:?}
            \n\
            The following actions will be taken:\n\
            {actions}
        ",
            os_type = "Linux",
            init_type = "systemd",
            nix_channels = "todo",
            actions = actions
                .iter()
                .map(|v| v.describe_execute())
                .flatten()
                .map(|desc| {
                    let ActionDescription {
                        description,
                        explanation,
                    } = desc;

                    let mut buf = String::default();
                    buf.push_str(&format!("* {description}\n"));
                    if explain {
                        for line in explanation {
                            buf.push_str(&format!("  {line}\n"));
                        }
                    }
                    buf
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    #[tracing::instrument(skip_all)]
    pub async fn install(&mut self) -> Result<(), HarmonicError> {
        let Self {
            actions,
            planner: _,
        } = self;

        // This is **deliberately sequential**.
        // Actions which are parallelizable are represented by "group actions" like CreateUsers
        // The plan itself represents the concept of the sequence of stages.
        for action in actions {
            if let Err(err) = action.execute().await {
                if let Err(err) = write_receipt(self.clone()).await {
                    tracing::error!("Error saving receipt: {:?}", err);
                }
                return Err(ActionError::from(err).into());
            }
        }

        write_receipt(self.clone()).await
    }

    #[tracing::instrument(skip_all)]
    pub fn describe_revert(&self, explain: bool) -> String {
        let Self { planner, actions } = self;
        format!(
            "\
            This Nix uninstall is for:\n\
              Operating System: {os_type}\n\
              Init system: {init_type}\n\
              Nix channels: {nix_channels}\n\
            \n\
            Created by planner: {planner:?}
            \n\
            The following actions will be taken:\n\
            {actions}
        ",
            os_type = "Linux",
            init_type = "systemd",
            nix_channels = "todo",
            actions = actions
                .iter()
                .map(|v| v.describe_revert())
                .flatten()
                .map(|desc| {
                    let ActionDescription {
                        description,
                        explanation,
                    } = desc;

                    let mut buf = String::default();
                    buf.push_str(&format!("* {description}\n"));
                    if explain {
                        for line in explanation {
                            buf.push_str(&format!("  {line}\n"));
                        }
                    }
                    buf
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    #[tracing::instrument(skip_all)]
    pub async fn revert(&mut self) -> Result<(), HarmonicError> {
        let Self {
            actions,
            planner: _,
        } = self;

        // This is **deliberately sequential**.
        // Actions which are parallelizable are represented by "group actions" like CreateUsers
        // The plan itself represents the concept of the sequence of stages.
        for action in actions {
            if let Err(err) = action.revert().await {
                if let Err(err) = write_receipt(self.clone()).await {
                    tracing::error!("Error saving receipt: {:?}", err);
                }
                return Err(ActionError::from(err).into());
            }
        }

        Ok(())
    }
}

async fn write_receipt(plan: InstallPlan) -> Result<(), HarmonicError> {
    tokio::fs::create_dir_all("/nix")
        .await
        .map_err(|e| HarmonicError::RecordingReceipt(PathBuf::from("/nix"), e))?;
    let install_receipt_path = PathBuf::from("/nix/receipt.json");
    let self_json =
        serde_json::to_string_pretty(&plan).map_err(HarmonicError::SerializingReceipt)?;
    tokio::fs::write(&install_receipt_path, self_json)
        .await
        .map_err(|e| HarmonicError::RecordingReceipt(install_receipt_path, e))?;
    Result::<(), HarmonicError>::Ok(())
}
