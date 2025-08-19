import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { useToast } from '@/hooks/use-toast';
import EmojiPicker from '@/components/ui/emoji-picker';
import { getDefaultIcon } from '@/utils/emoji-utils';

interface SaveArtifactDialogProps {
    isOpen: boolean;
    onClose: () => void;
    artifactType: string;
    code: string;
}

export default function SaveArtifactDialog({ 
    isOpen, 
    onClose, 
    artifactType, 
    code 
}: SaveArtifactDialogProps) {
    const [name, setName] = useState('');
    const [icon, setIcon] = useState(getDefaultIcon());
    const [description, setDescription] = useState('');
    const [tags, setTags] = useState('');
    const [isLoading, setIsLoading] = useState(false);
    const { toast } = useToast();

    const handleSave = async () => {
        if (!name.trim()) {
            toast({
                title: '错误',
                description: '请输入 artifact 名称',
                variant: 'destructive',
            });
            return;
        }

        setIsLoading(true);
        try {
            const request = {
                name: name.trim(),
                icon,
                description: description.trim(),
                artifact_type: artifactType,
                code,
                tags: tags.trim() || null,
            };

            await invoke<number>('save_artifact_to_collection', { request });
            
            toast({
                title: '保存成功',
                description: `Artifact "${name}" 已保存到合集中`,
            });

            // 重置表单
            setName('');
            setIcon(getDefaultIcon());
            setDescription('');
            setTags('');
            
            onClose();
        } catch (error) {
            console.error('保存失败:', error);
            toast({
                title: '保存失败',
                description: error as string,
                variant: 'destructive',
            });
        } finally {
            setIsLoading(false);
        }
    };

    const handleCancel = () => {
        // 重置表单
        setName('');
        setIcon(getDefaultIcon());
        setDescription('');
        setTags('');
        onClose();
    };

    // 当对话框关闭时重置表单
    React.useEffect(() => {
        if (!isOpen) {
            setName('');
            setIcon(getDefaultIcon());
            setDescription('');
            setTags('');
        }
    }, [isOpen]);

    return (
        <Dialog open={isOpen} onOpenChange={handleCancel}>
            <DialogContent className="sm:max-w-[525px]">
                <DialogHeader>
                    <DialogTitle>保存 Artifact 到合集</DialogTitle>
                    <DialogDescription>
                        将当前的 {artifactType.toUpperCase()} artifact 保存到您的合集中，方便以后快速访问。
                    </DialogDescription>
                </DialogHeader>
                
                <div className="grid gap-4 py-4">
                    {/* 图标选择 */}
                    <EmojiPicker 
                        value={icon} 
                        onChange={setIcon}
                    />

                    {/* 名称 */}
                    <div className="grid grid-cols-4 items-center gap-4">
                        <Label htmlFor="name" className="text-right">
                            名称 *
                        </Label>
                        <Input
                            id="name"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            className="col-span-3"
                            placeholder="输入 artifact 名称"
                            autoFocus
                        />
                    </div>

                    {/* 描述 */}
                    <div className="grid grid-cols-4 items-start gap-4">
                        <Label htmlFor="description" className="text-right pt-2">
                            描述
                        </Label>
                        <Textarea
                            id="description"
                            value={description}
                            onChange={(e) => setDescription(e.target.value)}
                            className="col-span-3"
                            placeholder="描述这个 artifact 的用途或特点..."
                            rows={3}
                        />
                    </div>

                    {/* 标签 */}
                    <div className="grid grid-cols-4 items-center gap-4">
                        <Label htmlFor="tags" className="text-right">
                            标签
                        </Label>
                        <Input
                            id="tags"
                            value={tags}
                            onChange={(e) => setTags(e.target.value)}
                            className="col-span-3"
                            placeholder="用逗号分隔多个标签，如: 图表,数据,可视化"
                        />
                    </div>

                    {/* 类型展示 */}
                    <div className="grid grid-cols-4 items-center gap-4">
                        <Label className="text-right">类型</Label>
                        <div className="col-span-3">
                            <Badge variant="secondary" className="text-sm">
                                {artifactType}
                            </Badge>
                        </div>
                    </div>

                    {/* 代码预览 */}
                    <div className="grid grid-cols-4 items-start gap-4">
                        <Label className="text-right pt-2">代码预览</Label>
                        <div className="col-span-3">
                            <pre className="bg-muted p-3 rounded text-xs max-h-32 overflow-y-auto">
                                {code}
                            </pre>
                        </div>
                    </div>
                </div>

                <DialogFooter>
                    <Button variant="outline" onClick={handleCancel}>
                        取消
                    </Button>
                    <Button onClick={handleSave} disabled={isLoading}>
                        {isLoading ? '保存中...' : '保存'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}