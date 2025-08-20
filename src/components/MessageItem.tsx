import React, { useMemo } from "react";
import ReactMarkdown from "react-markdown";
import ReasoningMessage from "./ReasoningMessage";
import ErrorMessage from "./message-item/ErrorMessage";
import MessageActionButtons from "./message-item/MessageActionButtons";
import ImageAttachments from "./message-item/ImageAttachments";
import RawTextRenderer from "./RawTextRenderer";
import { ShineBorder } from "./magicui/shine-border";
import { DEFAULT_SHINE_BORDER_CONFIG } from "@/utils/shineConfig";
import { Message, StreamEvent, MCPToolCallUpdateEvent } from "../data/Conversation";
import { usePerformanceMonitor, measureSync } from "../hooks/usePerformanceMonitor";
import { useCopyHandler } from "../hooks/useCopyHandler";
import { useCustomTagParser } from "../hooks/useCustomTagParser";
import { useMarkdownConfig } from "../hooks/useMarkdownConfig";
import { useMcpToolCallProcessor } from "../hooks/useMcpToolCallProcessor";
import { useDisplayConfig } from "../hooks/useDisplayConfig";

interface MessageItemProps {
    message: Message;
    streamEvent?: StreamEvent;
    onCodeRun?: (lang: string, code: string) => void;
    onMessageRegenerate?: () => void;
    onMessageEdit?: () => void;
    isReasoningExpanded?: boolean;
    onToggleReasoningExpand?: () => void;
    shouldShowShineBorder?: boolean;
    conversationId?: number; // Add conversation_id context
    mcpToolCallStates?: Map<number, MCPToolCallUpdateEvent>; // Add MCP states
}

const MessageItem = React.memo<MessageItemProps>(
    ({
        message,
        streamEvent,
        onCodeRun,
        onMessageRegenerate,
        onMessageEdit,
        isReasoningExpanded = false,
        onToggleReasoningExpand,
        shouldShowShineBorder = false,
        conversationId,
        mcpToolCallStates,
    }) => {
        // 性能监控
        usePerformanceMonitor(
            "MessageItem",
            [message.id, message.content, message.message_type, streamEvent?.is_done, isReasoningExpanded],
            false
        );

        const { copyIconState, handleCopy } = useCopyHandler(message.content);
        const { parseCustomTags } = useCustomTagParser();
        const { isUserMessageMarkdownEnabled } = useDisplayConfig();

        // 统一的 Markdown 配置，根据用户消息类型和配置决定是否禁用 Markdown 语法
        const isUserMessage = message.message_type === "user";
        const markdownConfig = useMarkdownConfig({
            onCodeRun,
            disableMarkdownSyntax: isUserMessage && !isUserMessageMarkdownEnabled,
        });

        const mcpProcessor = useMcpToolCallProcessor(markdownConfig, {
            conversationId,
            messageId: message.id,
            mcpToolCallStates,
        });

        // 处理自定义标签解析
        const markdownContent = useMemo(
            () => measureSync(`markdown-parsing-${message.id}`, () => parseCustomTags(message.content), false),
            [message.content, parseCustomTags, message.id]
        );

        // 渲染内容 - 根据用户消息类型和配置选择渲染方式
        const contentElement = useMemo(
            () =>
                measureSync(
                    `content-render-${message.id}`,
                    () => {
                        // 如果是用户消息且禁用了 Markdown 渲染，使用 RawTextRenderer
                        if (isUserMessage && !isUserMessageMarkdownEnabled) {
                            return <RawTextRenderer content={markdownContent} />;
                        }

                        // 否则使用统一的 ReactMarkdown 渲染
                        const element = (
                            <ReactMarkdown
                                children={markdownContent}
                                remarkPlugins={markdownConfig.remarkPlugins as any}
                                rehypePlugins={markdownConfig.rehypePlugins as any}
                                components={markdownConfig.markdownComponents}
                            />
                        );

                        // MCP 工具调用后处理
                        return mcpProcessor.processContent(markdownContent, element);
                    },
                    false
                ),
            [markdownContent, markdownConfig, mcpProcessor, message.id, isUserMessage, isUserMessageMarkdownEnabled]
        );

        // 早期返回：reasoning 类型消息
        if (message.message_type === "reasoning") {
            return (
                <ReasoningMessage
                    message={message}
                    streamEvent={streamEvent}
                    displayedContent={message.content}
                    isReasoningExpanded={isReasoningExpanded}
                    onToggleReasoningExpand={onToggleReasoningExpand}
                />
            );
        }

        // 早期返回：错误类型消息
        if (message.message_type === "error") {
            return <ErrorMessage content={message.content} />;
        }

        // 常规消息渲染
        return (
            <div
                className={`group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 bg-background text-foreground border border-border ${
                    isUserMessage ? "self-end" : "self-start"
                }`}
            >
                {shouldShowShineBorder && (
                    <ShineBorder
                        shineColor={DEFAULT_SHINE_BORDER_CONFIG.shineColor}
                        borderWidth={DEFAULT_SHINE_BORDER_CONFIG.borderWidth}
                        duration={DEFAULT_SHINE_BORDER_CONFIG.duration}
                    />
                )}

                <div className="prose prose-sm max-w-none text-foreground">
                    {/* RawTextRenderer 已包含 prose 样式，条件渲染避免重复包装 */}
                    {isUserMessage && !isUserMessageMarkdownEnabled ? contentElement : <div>{contentElement}</div>}
                </div>

                <ImageAttachments attachments={message.attachment_list} />

                <MessageActionButtons
                    messageType={message.message_type}
                    isUserMessage={isUserMessage}
                    copyIconState={copyIconState}
                    onCopy={handleCopy}
                    onEdit={onMessageEdit}
                    onRegenerate={onMessageRegenerate}
                />
            </div>
        );
    }
);

// 自定义比较函数，只在关键属性变化时才重新渲染
const areEqual = (prevProps: MessageItemProps, nextProps: MessageItemProps) => {
    // 基本消息属性比较
    if (prevProps.message.id !== nextProps.message.id) return false;
    if (prevProps.message.content !== nextProps.message.content) return false;
    if (prevProps.message.message_type !== nextProps.message.message_type) return false;

    // regenerate 数组比较
    const prevRegenerate = prevProps.message.regenerate;
    const nextRegenerate = nextProps.message.regenerate;
    if (prevRegenerate?.length !== nextRegenerate?.length) return false;

    // 流式事件比较
    const prevStreamEvent = prevProps.streamEvent;
    const nextStreamEvent = nextProps.streamEvent;
    if (prevStreamEvent?.is_done !== nextStreamEvent?.is_done) return false;
    if (prevStreamEvent?.content !== nextStreamEvent?.content) return false;

    // reasoning 展开状态比较
    if (prevProps.isReasoningExpanded !== nextProps.isReasoningExpanded) return false;

    // ShineBorder 动画状态比较
    if (prevProps.shouldShowShineBorder !== nextProps.shouldShowShineBorder) return false;

    // 回调函数比较（通常应该是稳定的）
    if (prevProps.onCodeRun !== nextProps.onCodeRun) return false;
    if (prevProps.onMessageRegenerate !== nextProps.onMessageRegenerate) return false;
    if (prevProps.onMessageEdit !== nextProps.onMessageEdit) return false;
    if (prevProps.onToggleReasoningExpand !== nextProps.onToggleReasoningExpand) return false;

    return true;
};

MessageItem.displayName = "MessageItem";

export default React.memo(MessageItem, areEqual);
