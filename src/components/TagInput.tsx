import React, { useState, KeyboardEvent, ChangeEvent, useCallback } from 'react';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Badge } from './ui/badge';
import { X, Tag } from 'lucide-react';

// 定义TagInputProps接口
interface TagInputProps {
    tags: string[];
    placeholder?: string;
    onAddTag: (tag: string) => void;
    onRemoveTag: (index: number) => void;
}

// TagInput组件
const TagInput: React.FC<TagInputProps> = ({ tags, placeholder, onAddTag, onRemoveTag }) => {
    const [inputValue, setInputValue] = useState<string>('');

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

    return (
        <div className="space-y-4">
            {/* 标签显示区域 */}
            {tags.length > 0 && (
                <div className="space-y-3">
                    <div className="flex items-center gap-2 text-sm text-gray-600">
                        <Tag className="h-4 w-4" />
                        <span className="font-medium">已配置模型 ({tags.length})</span>
                    </div>
                    <div className="flex flex-wrap gap-2 p-3 bg-gray-50 rounded-lg border border-gray-200 min-h-[60px]">
                        {tags.map((tag, index) => (
                            <Badge
                                key={index}
                                variant="secondary"
                                className="bg-gray-100 text-gray-800 border-gray-200 hover:bg-gray-200 transition-colors pl-3 pr-1 py-1 text-sm"
                            >
                                <span className="mr-2">{tag}</span>
                                <Button
                                    variant="ghost"
                                    size="sm"
                                    className="h-4 w-4 p-0 hover:bg-gray-300 hover:text-gray-900 rounded-full ml-1"
                                    onClick={() => onRemoveTag(index)}
                                >
                                    <X className="h-3 w-3" />
                                </Button>
                            </Badge>
                        ))}
                    </div>
                </div>
            )}

            {/* 输入框 */}
            <div className="space-y-2">
                <label className="text-sm font-medium text-gray-700">添加新模型</label>
                <Input
                    type="text"
                    value={inputValue}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    placeholder={placeholder || "输入模型名称，按回车确认"}
                    className="focus:ring-gray-500 focus:border-gray-500"
                />
                <p className="text-xs text-gray-500">
                    输入模型名称后按回车键添加，或点击标签上的 × 删除模型
                </p>
            </div>
        </div>
    );
};

export default TagInput;
