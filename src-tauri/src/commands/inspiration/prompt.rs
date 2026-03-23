use crate::agent_engine::messages::{AgentMessage, ContentBlock, ConversationState, Role};

const DEFAULT_INSPIRATION_SYSTEM_PROMPT: &str = r#"你是 Magic Novel 的灵感共创助手。

你的职责：
- 通过自然对话帮助用户把模糊灵感收束成可创建作品的方向。
- 优先澄清故事核心、主角、世界观、冲突、风格和读者感受。
- 不要假设项目已经存在，不要引用项目文件，不要要求 project_path。
- 未经用户确认，不要把候选想法当成最终定稿。
- 如果信息不足，优先提出一个最关键的问题，而不是一次抛出很多问题。
- 输出风格要像经验足够的创作编辑，直接、具体、克制。
- 允许调用轻量 inspiration 工具来更新共识或待确认问题。
- 禁止尝试调用任何项目读写类工具。"#;

pub fn apply_inspiration_system_prompt(state: &mut ConversationState, system_prompt: Option<&str>) {
    let effective = system_prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_INSPIRATION_SYSTEM_PROMPT)
        .to_string();

    if state.messages.is_empty() {
        state.messages.push(AgentMessage::system(effective));
        return;
    }

    let has_system_first = matches!(
        state.messages.first().map(|message| &message.role),
        Some(Role::System)
    );
    if !has_system_first {
        state.messages.insert(0, AgentMessage::system(effective));
        return;
    }

    if let Some(first) = state.messages.first_mut() {
        first.blocks = vec![ContentBlock::Text { text: effective }];
    }
}
