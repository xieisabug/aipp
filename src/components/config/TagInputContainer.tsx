import React, { useCallback, useState } from 'react';
import TagInput from '../TagInput';
import { invoke } from "@tauri-apps/api/core";
import { toast } from 'sonner';

interface TagInputContainerProps {
    llmProviderId: string;
    tags: string[];
    onTagsChange: (tags: string[]) => void;
    isExpanded?: boolean;
    onExpandedChange?: (expanded: boolean) => void;
}

const TagInputContainer: React.FC<TagInputContainerProps> = ({
    llmProviderId,
    tags,
    onTagsChange,
    isExpanded: externalIsExpanded,
    onExpandedChange
}) => {
    console.log("TagInputContainer render", { tags });
    const [internalIsExpanded, setInternalIsExpanded] = useState<boolean>(false);
    
    // 使用外部传入的展开状态，如果没有则使用内部状态
    const isExpanded = externalIsExpanded !== undefined ? externalIsExpanded : internalIsExpanded;
    const setIsExpanded = onExpandedChange || setInternalIsExpanded;

    // 添加模型
    const handleAddTag = useCallback((tag: string) => {
        invoke<Array<LLMModel>>('add_llm_model', { code: tag, llmProviderId })
            .then(() => {
                console.log("添加模型成功");
                onTagsChange([...tags, tag]);
            })
            .catch((e) => {
                console.log(e);
                toast.error('添加模型失败' + e);
            });
    }, [llmProviderId, tags, onTagsChange]);

    // 移除模型
    const handleRemoveTag = useCallback((index: number) => {
        const tagToRemove = tags[index];
        invoke<Array<LLMModel>>('delete_llm_model', { code: tagToRemove, llmProviderId })
            .then(() => {
                console.log("删除模型成功");
                onTagsChange(tags.filter((_, i) => i !== index));
            })
            .catch((e) => {
                console.log(e);
                toast.error('删除模型失败' + e);
            });
    }, [llmProviderId, tags, onTagsChange]);

    return (
        <TagInput
            placeholder="输入自定义Model按回车确认"
            tags={tags}
            onAddTag={handleAddTag}
            onRemoveTag={handleRemoveTag}
            isExpanded={isExpanded}
            onExpandedChange={setIsExpanded}
        />
    );
};

// 优化的比较函数，只在关键 props 变化时才重新渲染
export default React.memo(TagInputContainer, (prevProps, nextProps) => {
    return (
        prevProps.llmProviderId === nextProps.llmProviderId &&
        prevProps.tags.length === nextProps.tags.length &&
        prevProps.tags.every((tag, index) => tag === nextProps.tags[index]) &&
        prevProps.isExpanded === nextProps.isExpanded &&
        prevProps.onTagsChange === nextProps.onTagsChange &&
        prevProps.onExpandedChange === nextProps.onExpandedChange
    );
});