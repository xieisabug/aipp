import React, { useState, KeyboardEvent, ChangeEvent, useCallback, useEffect, useRef } from 'react';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Badge } from './ui/badge';
import { X, Tag, ChevronDown, ChevronUp } from 'lucide-react';

// 定义TagInputProps接口
interface TagInputProps {
    tags: string[];
    placeholder?: string;
    onAddTag: (tag: string) => void;
    onRemoveTag: (index: number) => void;
    isExpanded?: boolean;
    onExpandedChange?: (expanded: boolean) => void;
    onFetchModels?: () => void;
    isFetchingModels?: boolean;
}

// TagInput组件
const TagInput: React.FC<TagInputProps> = ({ 
    tags, 
    placeholder, 
    onAddTag, 
    onRemoveTag, 
    isExpanded: externalIsExpanded,
    onExpandedChange,
    onFetchModels,
    isFetchingModels = false
}) => {
    const [inputValue, setInputValue] = useState<string>('');
    const [internalIsExpanded, setInternalIsExpanded] = useState<boolean>(false);
    const [shouldShowExpandButton, setShouldShowExpandButton] = useState<boolean>(false);
    
    // 使用外部传入的展开状态，如果没有则使用内部状态
    const isExpanded = externalIsExpanded !== undefined ? externalIsExpanded : internalIsExpanded;
    const setIsExpanded = onExpandedChange || setInternalIsExpanded;
    const tagsContainerRef = useRef<HTMLDivElement>(null);

    const handleKeyDown = useCallback((e: KeyboardEvent<HTMLInputElement>) => {
        if (e.key === 'Enter' && inputValue.trim() !== '') {
            console.log("TagInput handleKeyDown", inputValue);
            onAddTag(inputValue.trim());
            setInputValue('');
        }
    }, [inputValue, onAddTag]);

    const handleChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
        setInputValue(e.target.value);
    }, []);

    // 检测是否需要显示展开按钮
    useEffect(() => {
        if (tags.length > 0 && tagsContainerRef.current) {
            // 计算标签容器的实际高度
            const container = tagsContainerRef.current;
            const containerHeight = container.scrollHeight;
            // 大概两排半的高度（每个标签约32px高 + gap，两排半约110px）
            const twoAndHalfRowsHeight = 110;

            setShouldShowExpandButton(containerHeight > twoAndHalfRowsHeight);

            // 如果标签数量减少，可能不再需要收起状态
            if (containerHeight <= twoAndHalfRowsHeight) {
                setIsExpanded(false);
            }
        } else {
            setShouldShowExpandButton(false);
            setIsExpanded(false);
        }
    }, [tags]);

    const toggleExpansion = useCallback(() => {
        setIsExpanded(!isExpanded);
    }, [isExpanded, setIsExpanded]);

    return (
        <div className="space-y-4">
            {/* 标签显示区域或获取按钮 */}
            {(tags.length > 0 || onFetchModels) && (
                <div className="space-y-3">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                            <Tag className="h-4 w-4" />
                            <span className="font-medium">
                                {tags.length > 0 ? `已配置模型 (${tags.length})` : "模型列表"}
                            </span>
                        </div>
                        <div className="flex items-center gap-2">
                            {onFetchModels && (
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={onFetchModels}
                                    disabled={isFetchingModels}
                                    className="h-6 px-2 text-xs hover:bg-muted hover:border-muted-foreground"
                                >
                                    {isFetchingModels ? "获取中..." : "获取Model列表"}
                                </Button>
                            )}
                            {shouldShowExpandButton && (
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    onClick={toggleExpansion}
                                    className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground hover:bg-muted"
                                >
                                    {isExpanded ? (
                                        <>
                                            <ChevronUp className="h-3 w-3 mr-1" />
                                            收起
                                        </>
                                    ) : (
                                        <>
                                            <ChevronDown className="h-3 w-3 mr-1" />
                                            展开
                                        </>
                                    )}
                                </Button>
                            )}
                        </div>
                    </div>
                    {tags.length > 0 && (
                        <div className="relative">
                            <div
                                ref={tagsContainerRef}
                                className={`
                                    flex flex-wrap gap-2 p-3 bg-muted rounded-lg border border-border 
                                    transition-all duration-300 ease-in-out
                                    ${shouldShowExpandButton && !isExpanded
                                        ? 'max-h-[110px] overflow-hidden'
                                        : 'max-h-none'
                                    }
                                `}
                                style={{ minHeight: tags.length > 0 ? '60px' : undefined }}
                            >
                                {tags.map((tag, index) => (
                                <Badge
                                    key={index}
                                    variant="secondary"
                                    className="bg-muted text-foreground border-border hover:bg-muted/80 transition-colors pl-3 pr-1 py-1 text-sm"
                                >
                                    <span className="mr-2">{tag}</span>
                                    <Button
                                        variant="ghost"
                                        size="sm"
                                        className="h-4 w-4 p-0 hover:bg-muted-foreground/20 hover:text-foreground rounded-full ml-1"
                                        onClick={() => onRemoveTag(index)}
                                    >
                                        <X className="h-3 w-3" />
                                    </Button>
                                </Badge>
                            ))}
                        </div>

                        {/* 渐变遮罩效果和底部展开区域，当收起时显示 */}
                        {shouldShowExpandButton && !isExpanded && (
                            <>
                                {/* 渐变遮罩 */}
                                <div className="absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-muted to-transparent pointer-events-none rounded-b-lg" />

                                {/* 底部可点击展开区域 */}
                                <div
                                    onClick={toggleExpansion}
                                    className="absolute bottom-0 left-0 right-0 h-8 flex items-center justify-center cursor-pointer hover:bg-muted/80 rounded-b-lg transition-colors group"
                                    title="点击展开查看更多模型"
                                >
                                    <div className="flex items-center gap-1 text-xs text-muted-foreground group-hover:text-foreground">
                                        <ChevronDown className="h-3 w-3" />
                                        <span>展开更多</span>
                                    </div>
                                </div>
                            </>
                        )}

                        {/* 展开状态下的收起区域 */}
                        {shouldShowExpandButton && isExpanded && (
                            <div className="mt-2 pt-2 border-t border-border">
                                <div
                                    onClick={toggleExpansion}
                                    className="flex items-center justify-center cursor-pointer hover:bg-muted rounded-md py-1 transition-colors group"
                                    title="点击收起模型列表"
                                >
                                    <div className="flex items-center gap-1 text-xs text-muted-foreground group-hover:text-foreground">
                                        <ChevronUp className="h-3 w-3" />
                                        <span>收起</span>
                                    </div>
                                </div>
                            </div>
                        )}
                        </div>
                    )}
                </div>
            )}

            {/* 输入框 */}
            <div className="space-y-2">
                <label className="text-sm font-medium text-foreground">添加新模型</label>
                <Input
                    type="text"
                    value={inputValue}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    placeholder={placeholder || "输入模型名称，按回车确认"}
                    className="focus:ring-primary focus:border-primary"
                />
                <p className="text-xs text-muted-foreground">
                    输入模型名称后按回车键添加，或点击标签上的 × 删除模型
                </p>
            </div>
        </div>
    );
};

export default TagInput;
