import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useForm } from 'react-hook-form';
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
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
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
    const [isLoading, setIsLoading] = useState(false);
    const { toast } = useToast();

    const form = useForm({
        defaultValues: {
            name: '',
            icon: getDefaultIcon(),
            description: '',
            tags: '',
        }
    });

    const handleSave = async (data: any) => {
        setIsLoading(true);
        try {
            const request = {
                name: data.name.trim(),
                icon: data.icon,
                description: data.description.trim(),
                artifact_type: artifactType,
                code,
                tags: data.tags.trim() || null,
            };

            await invoke<number>('save_artifact_to_collection', { request });
            
            toast({
                title: '保存成功',
                description: `Artifact "${data.name}" 已保存到合集中`,
            });

            form.reset();
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
        form.reset();
        onClose();
    };

    // 当对话框关闭时重置表单
    React.useEffect(() => {
        if (!isOpen) {
            form.reset();
        }
    }, [isOpen, form]);

    return (
        <Dialog open={isOpen} onOpenChange={handleCancel}>
            <DialogContent className="sm:max-w-[525px] max-h-[80vh] overflow-y-auto">
                <DialogHeader>
                    <DialogTitle>保存 Artifact 到合集</DialogTitle>
                    <DialogDescription>
                        将当前的 {artifactType.toUpperCase()} artifact 保存到您的合集中，方便以后快速访问。
                    </DialogDescription>
                </DialogHeader>
                
                <Form {...form}>
                    <form onSubmit={form.handleSubmit(handleSave)} className="space-y-6 py-4">
                        {/* 图标选择 */}
                        <FormField
                            control={form.control}
                            name="icon"
                            render={({ field }) => (
                                <FormItem className="space-y-3">
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                        图标
                                    </FormLabel>
                                    <FormControl>
                                        <EmojiPicker 
                                            className="focus:ring-ring/20 focus:border-ring"
                                            value={field.value}
                                            onChange={field.onChange}
                                        />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />

                        {/* 名称 */}
                        <FormField
                            control={form.control}
                            name="name"
                            rules={{ required: '请输入 artifact 名称' }}
                            render={({ field }) => (
                                <FormItem className="space-y-3">
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                        名称 *
                                    </FormLabel>
                                    <FormControl>
                                        <Input
                                            className="focus:ring-ring/20 focus:border-ring"
                                            placeholder="输入 artifact 名称"
                                            autoFocus
                                            {...field}
                                        />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />

                        {/* 描述 */}
                        <FormField
                            control={form.control}
                            name="description"
                            render={({ field }) => (
                                <FormItem className="space-y-3">
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                        描述
                                    </FormLabel>
                                    <FormControl>
                                        <Textarea
                                            className="focus:ring-ring/20 focus:border-ring"
                                            placeholder="描述这个 artifact 的用途或特点..."
                                            rows={3}
                                            {...field}
                                        />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />

                        {/* 标签 */}
                        <FormField
                            control={form.control}
                            name="tags"
                            render={({ field }) => (
                                <FormItem className="space-y-3">
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                        标签
                                    </FormLabel>
                                    <FormControl>
                                        <Input
                                            className="focus:ring-ring/20 focus:border-ring"
                                            placeholder="用逗号分隔多个标签，如: 图表,数据,可视化"
                                            {...field}
                                        />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />

                        {/* 类型展示 */}
                        <FormItem className="space-y-3">
                            <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                类型
                            </FormLabel>
                            <FormControl>
                                <div className="px-3 py-2 bg-muted rounded-md">
                                    <Badge variant="secondary" className="text-sm">
                                        {artifactType}
                                    </Badge>
                                </div>
                            </FormControl>
                        </FormItem>

                        {/* 代码预览 */}
                        <FormItem className="space-y-3">
                            <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                                代码预览
                            </FormLabel>
                            <FormControl>
                                <pre className="bg-muted p-3 rounded-md text-xs max-h-32 overflow-y-auto border">
                                    {code}
                                </pre>
                            </FormControl>
                        </FormItem>
                    </form>
                </Form>

                <DialogFooter>
                    <Button variant="outline" onClick={handleCancel}>
                        取消
                    </Button>
                    <Button 
                        onClick={form.handleSubmit(handleSave)} 
                        disabled={isLoading}
                    >
                        {isLoading ? '保存中...' : '保存'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}