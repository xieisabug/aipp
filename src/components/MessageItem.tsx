import React, { useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import ReasoningMessage from './ReasoningMessage';
import ErrorMessage from './MessageItem/ErrorMessage';
import MessageActionButtons from './MessageItem/MessageActionButtons';
import ImageAttachments from './MessageItem/ImageAttachments';
import { ShineBorder } from './magicui/shine-border';
import { Message, StreamEvent } from '../data/Conversation';
import { usePerformanceMonitor, measureSync } from '../hooks/usePerformanceMonitor';
import { useCopyHandler } from '../hooks/useCopyHandler';
import { useCustomTagParser } from '../hooks/useCustomTagParser';
import { useMarkdownConfig } from '../hooks/useMarkdownConfig';
import { useMcpToolCallProcessor } from '../hooks/useMcpToolCallProcessor';

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
}

const MessageItem = React.memo<MessageItemProps>(({
    message,
    streamEvent,
    onCodeRun,
    onMessageRegenerate,
    onMessageEdit,
    isReasoningExpanded = false,
    onToggleReasoningExpand,
    shouldShowShineBorder = false,
    conversationId,
}) => {
    // 性能监控
    usePerformanceMonitor(
        'MessageItem',
        [
            message.id,
            message.content,
            message.message_type,
            streamEvent?.is_done,
            isReasoningExpanded,
        ],
        false,
    );

    // Hooks
    const { copyIconState, handleCopy } = useCopyHandler(message.content);
    const { parseCustomTags } = useCustomTagParser();
    const markdownConfig = useMarkdownConfig({ onCodeRun });
    const mcpProcessor = useMcpToolCallProcessor(markdownConfig, { 
        conversationId, 
        messageId: message.id 
    });

    // 处理自定义标签解析
    const markdownContent = useMemo(
        () => measureSync(
            `markdown-parsing-${message.id}`,
            () => parseCustomTags(message.content),
            false,
        ),
        [message.content, parseCustomTags, message.id],
    );

    // 渲染 Markdown 内容
    const markdownElement = useMemo(
        () => measureSync(
            `markdown-render-${message.id}`,
            () => {
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
            false,
        ),
        [markdownContent, markdownConfig, mcpProcessor, message.id],
    );

    // 早期返回：reasoning 类型消息
    if (message.message_type === 'reasoning') {
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
    if (message.message_type === 'error') {
        return <ErrorMessage content={message.content} />;
    }

    // 常规消息渲染
    const isUserMessage = message.message_type === 'user';

    return (
        <div
            className={`group relative py-4 px-5 rounded-2xl inline-block max-w-[65%] transition-all duration-200 ${isUserMessage
                    ? 'self-end bg-secondary text-primary'
                    : 'self-start bg-background text-foreground border border-border'
                }`}
        >
            {shouldShowShineBorder && (
                <ShineBorder
                    shineColor={['#A07CFE', '#FE8FB5', '#FFBE7B']}
                    borderWidth={2}
                    duration={8}
                />
            )}

            <div className="prose prose-sm max-w-none">
                {markdownElement}
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
});

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

MessageItem.displayName = 'MessageItem';

export default React.memo(MessageItem, areEqual);