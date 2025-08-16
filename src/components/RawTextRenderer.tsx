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

        // 如果有自定义标签，需要分离处理
        const parts: React.ReactNode[] = [];
        let lastIndex = 0;
        
        // 重置正则表达式的 lastIndex
        customTagPattern.lastIndex = 0;
        let match;

        while ((match = customTagPattern.exec(content)) !== null) {
            // 添加标签前的纯文本
            if (match.index > lastIndex) {
                const textBefore = content.slice(lastIndex, match.index);
                if (textBefore) {
                    parts.push(
                        <span key={`text-${lastIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                            {textBefore}
                        </span>
                    );
                }
            }

            // 添加自定义标签（通过 ReactMarkdown 处理）
            const tagContent = match[0];
            parts.push(
                <ReactMarkdown
                    key={`tag-${match.index}`}
                    children={tagContent}
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

            lastIndex = match.index + match[0].length;
        }

        // 添加最后剩余的纯文本
        if (lastIndex < content.length) {
            const textAfter = content.slice(lastIndex);
            if (textAfter) {
                parts.push(
                    <span key={`text-${lastIndex}`} style={{ whiteSpace: 'pre-wrap' }}>
                        {textAfter}
                    </span>
                );
            }
        }

        return <>{parts}</>;
    }, [content]);

    return <div className="prose prose-sm max-w-none">{processedContent}</div>;
};

export default RawTextRenderer;