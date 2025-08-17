import React, { useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import { MARKDOWN_COMPONENTS_BASE, REMARK_PLUGINS, REHYPE_PLUGINS } from '@/constants/markdown';

interface RawTextRendererProps {
    content: string;
}

/**
 * RawTextRenderer - 用于在非 Markdown 模式下渲染文本
 * 保持文本的原始格式（换行、空行等），同时支持自定义标签的处理
 */
const RawTextRenderer: React.FC<RawTextRendererProps> = ({ content }) => {
    const processedContent = useMemo(() => {
        // 先检查是否包含自定义标签
        const customTagPattern = /<(fileattachment|bangwebtomarkdown|bangweb|tipscomponent)\b[^>]*>.*?<\/\1>|<(fileattachment|bangwebtomarkdown|bangweb|tipscomponent)\b[^>]*\/>/gi;
        const hasCustomTags = customTagPattern.test(content);

        if (!hasCustomTags) {
            // 如果没有自定义标签，直接以原始文本格式渲染
            return (
                <span style={{ whiteSpace: 'pre-wrap' }}>
                    {content}
                </span>
            );
        }

        // 重新设计分割逻辑：使用 split 来更精确地分割内容
        const parts: React.ReactNode[] = [];
        
        // 先找到所有标签的位置和内容
        customTagPattern.lastIndex = 0;
        const matches: Array<{ index: number; length: number; content: string }> = [];
        let match;
        
        while ((match = customTagPattern.exec(content)) !== null) {
            matches.push({
                index: match.index,
                length: match[0].length,
                content: match[0]
            });
        }

        let currentIndex = 0;
        
        matches.forEach((tagMatch) => {
            // 处理标签前的文本
            if (tagMatch.index > currentIndex) {
                const textBefore = content.slice(currentIndex, tagMatch.index);
                if (textBefore) {
                    parts.push(
                        <span key={`text-${currentIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                            {textBefore}
                        </span>
                    );
                }
            }

            // 处理自定义标签
            parts.push(
                <ReactMarkdown
                    key={`tag-${tagMatch.index}`}
                    children={tagMatch.content}
                    remarkPlugins={[
                        REMARK_PLUGINS.find(plugin => plugin.name === 'remarkCustomCompenent') || REMARK_PLUGINS[3]
                    ].filter(Boolean) as any}
                    rehypePlugins={[
                        REHYPE_PLUGINS[0], // rehypeRaw
                        REHYPE_PLUGINS[1], // rehypeSanitize
                    ] as any}
                    components={MARKDOWN_COMPONENTS_BASE as any}
                />
            );

            currentIndex = tagMatch.index + tagMatch.length;
        });

        // 处理最后剩余的文本
        if (currentIndex < content.length) {
            const textAfter = content.slice(currentIndex);
            
            // 关键改进：检查是否为紧跟标签的单个换行符
            const isSingleNewlineAfterTag = matches.length > 0 && /^\n/.test(textAfter);
            
            if (isSingleNewlineAfterTag) {
                // 如果是标签后的单个换行符，移除它，但保留后续内容
                const contentAfterNewline = textAfter.slice(1);
                if (contentAfterNewline) {
                    parts.push(
                        <span key={`text-${currentIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                            {contentAfterNewline}
                        </span>
                    );
                }
            } else if (textAfter) {
                // 其他情况正常渲染
                parts.push(
                    <span key={`text-${currentIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                        {textAfter}
                    </span>
                );
            }
        }

        return <>{parts}</>;
    }, [content]);

    return <div className="prose prose-sm max-w-none prose-neutral dark:prose-invert">{processedContent}</div>;
};

export default RawTextRenderer;