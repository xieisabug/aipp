import { useEffect, useState } from 'react';
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
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { useToast } from '@/hooks/use-toast';
import EmojiPicker from '@/components/ui/emoji-picker';

interface ArtifactForEdit {
    id: number;
    name: string;
    icon: string;
    description: string;
    tags?: string;
}

interface EditArtifactDialogProps {
    isOpen: boolean;
    onClose: () => void;
    artifact: ArtifactForEdit | null;
    onUpdated: () => void;
}

export default function EditArtifactDialog({ isOpen, onClose, artifact, onUpdated }: EditArtifactDialogProps) {
    const [isLoading, setIsLoading] = useState(false);
    const { toast } = useToast();

    const form = useForm<{ name: string; icon: string; description: string; tags: string }>({
        defaultValues: {
            name: artifact?.name ?? '',
            icon: artifact?.icon ?? '',
            description: artifact?.description ?? '',
            tags: artifact?.tags ?? '',
        },
    });

    // Sync form with incoming artifact or open state
    useEffect(() => {
        if (isOpen && artifact) {
            form.reset({
                name: artifact.name || '',
                icon: artifact.icon || '',
                description: artifact.description || '',
                tags: artifact.tags || '',
            });
        }
    }, [isOpen, artifact, form]);

    const handleSave = async (data: { name: string; icon: string; description: string; tags: string }) => {
        if (!artifact) return;
        setIsLoading(true);
        try {
            await invoke('update_artifact_collection', {
                request: {
                    id: artifact.id,
                    name: data.name.trim() || null,
                    icon: data.icon || null,
                    description: data.description.trim() || null,
                    tags: data.tags.trim() || null,
                },
            });

            toast({
                title: '更新成功',
                description: 'Artifact 信息已更新',
            });

            form.reset();
            onClose();
            onUpdated();
        } catch (error) {
            console.error('更新失败:', error);
            toast({
                title: '更新失败',
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

    // Reset form when dialog closes
    useEffect(() => {
        if (!isOpen) {
            form.reset();
        }
    }, [isOpen, form]);

    return (
        <Dialog
            open={isOpen}
            onOpenChange={(open) => {
                if (!open) handleCancel();
            }}
        >
            <DialogContent
                className="sm:max-w-[525px] max-h-[80vh] overflow-y-auto"

                onInteractOutside={(e) => {
                    const target = e.target as HTMLElement | null;
                    // Try to detect via composedPath first for reliability across WebView2
                    // @ts-ignore
                    const original = e?.detail?.originalEvent as PointerEvent | MouseEvent | undefined;
                    const path: EventTarget[] = (original && typeof (original as any).composedPath === 'function')
                        ? (original as any).composedPath()
                        : (target ? [target, ...(function collect(n: HTMLElement | null, acc: HTMLElement[] = []) {
                            if (!n) return acc;
                            acc.push(n);
                            return collect(n.parentElement, acc);
                        })(target)] : []);
                    const inEmoji = path.some((n) =>
                        n instanceof HTMLElement && (n.closest?.('[data-emoji-picker="true"]') || n.closest?.('[data-emoji-panel="true"]'))
                    );
                    if (inEmoji) e.preventDefault();
                }}
                onPointerDownOutside={(e) => {
                    const target = (e.target as HTMLElement) || null;
                    // @ts-ignore
                    const original = e?.detail?.originalEvent as PointerEvent | MouseEvent | undefined;
                    const path: EventTarget[] = (original && typeof (original as any).composedPath === 'function')
                        ? (original as any).composedPath()
                        : (target ? [target, ...(function collect(n: HTMLElement | null, acc: HTMLElement[] = []) {
                            if (!n) return acc;
                            acc.push(n);
                            return collect(n.parentElement, acc);
                        })(target)] : []);
                    const inEmoji = path.some((n) =>
                        n instanceof HTMLElement && (n.closest?.('[data-emoji-picker="true"]') || n.closest?.('[data-emoji-panel="true"]'))
                    );
                    if (inEmoji) e.preventDefault();
                }}
            >
                <DialogHeader>
                    <DialogTitle>编辑 Artifact</DialogTitle>
                    <DialogDescription>修改 artifact 的基本信息</DialogDescription>
                </DialogHeader>

                <Form {...form}>
                    <form onSubmit={form.handleSubmit(handleSave)} className="space-y-6 py-4">
                        {/* 图标选择 */}
                        <FormField
                            control={form.control}
                            name="icon"
                            render={({ field }) => (
                                <FormItem className="space-y-3">
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">图标</FormLabel>
                                    <FormControl>
                                        <EmojiPicker className="focus:ring-ring/20 focus:border-ring" value={field.value} onChange={field.onChange} />
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
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">名称 *</FormLabel>
                                    <FormControl>
                                        <Input className="focus:ring-ring/20 focus:border-ring" placeholder="输入 artifact 名称" autoFocus {...field} />
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
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">描述</FormLabel>
                                    <FormControl>
                                        <Textarea className="focus:ring-ring/20 focus:border-ring" placeholder="描述这个 artifact 的用途或特点..." rows={3} {...field} />
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
                                    <FormLabel className="flex items-center font-semibold text-sm text-foreground">标签</FormLabel>
                                    <FormControl>
                                        <Input className="focus:ring-ring/20 focus:border-ring" placeholder="用逗号分隔多个标签，如: 图表,数据,可视化" {...field} />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />
                    </form>
                </Form>

                <DialogFooter>
                    <Button variant="outline" onClick={handleCancel}>
                        取消
                    </Button>
                    <Button onClick={form.handleSubmit(handleSave)} disabled={isLoading}>
                        {isLoading ? '保存中...' : '保存'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
