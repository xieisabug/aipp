import React, { useCallback, useState } from 'react';
import TagInput from '../TagInput';
import { invoke } from "@tauri-apps/api/core";
import { toast } from 'sonner';

interface ModelForSelection {
    name: string;
    code: string;
    description: string;
    vision_support: boolean;
    audio_support: boolean;
    video_support: boolean;
    is_selected: boolean;
}

interface ModelSelectionResponse {
    available_models: ModelForSelection[];
    missing_models: string[];
}

interface TagInputContainerProps {
    llmProviderId: string;
    tags: string[];
    onTagsChange: (tags: string[]) => void;
    isExpanded?: boolean;
    onExpandedChange?: (expanded: boolean) => void;
    onFetchModels?: (modelData: ModelSelectionResponse) => void;
}

const TagInputContainer: React.FC<TagInputContainerProps> = ({
    llmProviderId,
    tags,
    onTagsChange,
    isExpanded: externalIsExpanded,
    onExpandedChange,
    onFetchModels
}) => {
    const [internalIsExpanded, setInternalIsExpanded] = useState<boolean>(false);
    const [isFetchingModels, setIsFetchingModels] = useState<boolean>(false);
    
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

    // 获取模型列表
    const handleFetchModels = useCallback(async () => {
        if (!onFetchModels) return;
        
        setIsFetchingModels(true);
        try {
            const modelData = await invoke<ModelSelectionResponse>("preview_model_list", { 
                llmProviderId: parseInt(llmProviderId) 
            });
            onFetchModels(modelData);
        } catch (e) {
            toast.error(
                "获取模型列表失败，请检查Endpoint和Api Key配置: " + e,
            );
        } finally {
            setIsFetchingModels(false);
        }
    }, [llmProviderId, onFetchModels]);

    return (
        <TagInput
            placeholder="输入自定义Model按回车确认"
            tags={tags}
            onAddTag={handleAddTag}
            onRemoveTag={handleRemoveTag}
            isExpanded={isExpanded}
            onExpandedChange={setIsExpanded}
            onFetchModels={onFetchModels ? handleFetchModels : undefined}
            isFetchingModels={isFetchingModels}
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