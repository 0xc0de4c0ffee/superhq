use super::{secret_entry, AgentConfig, InstallStep};

pub fn config() -> AgentConfig {
    AgentConfig {
        name: "pi",
        display_name: "Pi",
        command: "/usr/local/bin/pi",
        icon: Some("icons/agents/pi.svg"),
        color: Some("#FFFFFF"),
        tab_order: 3,
        install_steps: vec![
            InstallStep::Group {
                label: "Downloading Pi",
                skip_if: Some("/usr/local/bin/pi --version"),
                steps: vec![
                    InstallStep::Download {
                        label: "Downloading Pi",
                        url: "https://github.com/badlogic/pi-mono/releases/latest/download/pi-linux-arm64.tar.gz",
                        path: "/tmp",
                        extract: true,
                        skip_if: None,
                    },
                    InstallStep::Cmd {
                        label: "Installing binary",
                        command: "cp -r /tmp/pi/* /usr/local/bin/ && rm -rf /tmp/pi",
                        skip_if: None,
                    },
                    InstallStep::Chmod {
                        label: "Setting permissions",
                        path: "/usr/local/bin/pi",
                        mode: 0o755,
                        skip_if: None,
                    },
                ],
            },
            InstallStep::Cmd {
                label: "Verifying installation",
                command: "/usr/local/bin/pi --version",
                skip_if: None,
            },
        ],
        secrets: vec![secret_entry(
            "ANTHROPIC_API_KEY",
            "Anthropic API Key",
            &["api.anthropic.com"],
            &[],
        )],
        auth_gateway: None,
    }
}
