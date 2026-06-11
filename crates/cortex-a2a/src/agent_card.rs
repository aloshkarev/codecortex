use crate::roles::AgentRole;
use crate::wire::{
    AgentCapabilitiesWire, AgentCardWire, AgentExtensionWire, AgentInterfaceWire, AgentSkillWire,
    EXTENSION_INTELLIGENCE_COOPERATION,
};
use cortex_core::A2aConfig;

pub fn gateway_agent_card(config: &A2aConfig, base_url: &str) -> AgentCardWire {
    let mut interfaces = vec![AgentInterfaceWire {
        url: format!("{base_url}{}", config.server.base_path),
        protocol_binding: "HTTP+JSON".to_string(),
        protocol_version: config.server.protocol_version.clone(),
        tenant: None,
    }];
    if config.server.grpc_enabled {
        interfaces.push(AgentInterfaceWire {
            url: format!("http://{}", config.server.grpc_listen),
            protocol_binding: "GRPC".to_string(),
            protocol_version: config.server.protocol_version.clone(),
            tenant: None,
        });
    }
    AgentCardWire {
        name: "CodeCortex Gateway".to_string(),
        description: "Orchestrates hybrid A2A sessions over the code graph blackboard.".to_string(),
        supported_interfaces: interfaces,
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: AgentCapabilitiesWire {
            streaming: Some(true),
            push_notifications: Some(config.push.enabled),
        },
        default_input_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        default_output_modes: vec!["application/json".to_string()],
        skills: vec![AgentSkillWire {
            id: "spawn_session".to_string(),
            name: "Spawn A2A session".to_string(),
            description: "Start a multi-agent workflow with return_immediately task semantics."
                .to_string(),
            tags: vec!["orchestration".to_string(), "codecortex".to_string()],
            examples: vec![],
        }],
        extensions: vec![AgentExtensionWire {
            uri: EXTENSION_INTELLIGENCE_COOPERATION.to_string(),
            required: false,
        }],
    }
}

pub fn role_agent_card(
    role: AgentRole,
    config: &A2aConfig,
    base_url: &str,
    mcp_tools: &[String],
) -> AgentCardWire {
    let path = format!(
        "{}/.well-known/agents/{}.json",
        base_url.trim_end_matches('/'),
        role.as_str()
    );
    let mut skills: Vec<AgentSkillWire> = mcp_tools
        .iter()
        .map(|tool| AgentSkillWire {
            id: tool.clone(),
            name: tool.clone(),
            description: format!("CodeCortex MCP tool `{tool}` for role {role}"),
            tags: vec![
                "mcp".to_string(),
                "codecortex".to_string(),
                role.as_str().to_string(),
            ],
            examples: mcp_tool_example(tool),
        })
        .collect();
    if skills.is_empty() {
        skills.push(AgentSkillWire {
            id: role.as_str().to_string(),
            name: role.to_string(),
            description: format!("{role} role in CodeCortex hybrid topology"),
            tags: vec!["codecortex".to_string(), role.as_str().to_string()],
            examples: vec![],
        });
    }
    AgentCardWire {
        name: format!("CodeCortex {}", role),
        description: format!("CodeCortex A2A role: {role}"),
        supported_interfaces: vec![AgentInterfaceWire {
            url: path,
            protocol_binding: "HTTP+JSON".to_string(),
            protocol_version: config.server.protocol_version.clone(),
            tenant: Some(role.as_str().to_string()),
        }],
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: AgentCapabilitiesWire {
            streaming: Some(true),
            push_notifications: None,
        },
        default_input_modes: vec!["application/json".to_string()],
        default_output_modes: vec!["application/json".to_string()],
        skills,
        extensions: vec![AgentExtensionWire {
            uri: EXTENSION_INTELLIGENCE_COOPERATION.to_string(),
            required: false,
        }],
    }
}

fn mcp_tool_example(tool: &str) -> Vec<String> {
    match tool {
        "get_patch_context" => vec![
            r#"{"task":"fix auth retry","includePaths":["src/auth"],"budgetTokens":6000}"#
                .to_string(),
        ],
        "get_impact_graph" => vec![r#"{"symbol":"AuthClient::refresh","depth":4}"#.to_string()],
        "get_delta_context" => vec![
            r#"{"sourceBranch":"HEAD","targetBranch":"main","includePaths":["crates/"]}"#
                .to_string(),
        ],
        "get_api_contract" => vec![r#"{"path":"src/handler.rs","budgetTokens":4000}"#.to_string()],
        "get_test_context" => vec![r#"{"symbol":"AuthClient","budgetTokens":4000}"#.to_string()],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roles::AgentRole;

    #[test]
    fn gateway_lists_cooperation_extension() {
        let card = gateway_agent_card(&A2aConfig::default(), "http://127.0.0.1:8080");
        assert!(
            card.extensions
                .iter()
                .any(|e| e.uri == EXTENSION_INTELLIGENCE_COOPERATION)
        );
    }

    #[test]
    fn patch_planner_skill_tags_include_mcp_tool() {
        let card = role_agent_card(
            AgentRole::PatchPlanner,
            &A2aConfig::default(),
            "http://127.0.0.1:8080",
            &["get_patch_context".to_string()],
        );
        let skill = card
            .skills
            .iter()
            .find(|s| s.id == "get_patch_context")
            .expect("patch context skill");
        assert!(skill.tags.contains(&"mcp".to_string()));
    }
}
