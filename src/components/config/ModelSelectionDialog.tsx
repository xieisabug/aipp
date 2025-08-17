import React, { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { AlertTriangle, Eye, Mic, Video } from 'lucide-react';

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

interface ModelSelectionDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    modelData: ModelSelectionResponse | null;
    onConfirm: (selectedModels: ModelForSelection[]) => void;
    loading: boolean;
}

const ModelSelectionDialog: React.FC<ModelSelectionDialogProps> = ({
    open,
    onOpenChange,
    modelData,
    onConfirm,
    loading
}) => {
    const [selectedModels, setSelectedModels] = useState<ModelForSelection[]>([]);

    React.useEffect(() => {
        if (modelData) {
            setSelectedModels(modelData.available_models);
        }
    }, [modelData]);

    const handleModelToggle = (modelCode: string, checked: boolean) => {
        setSelectedModels(prev => 
            prev.map(model => 
                model.code === modelCode 
                    ? { ...model, is_selected: checked }
                    : model
            )
        );
    };

    const handleSelectAll = () => {
        setSelectedModels(prev => 
            prev.map(model => ({ ...model, is_selected: true }))
        );
    };

    const handleDeselectAll = () => {
        setSelectedModels(prev => 
            prev.map(model => ({ ...model, is_selected: false }))
        );
    };

    const handleConfirm = () => {
        onConfirm(selectedModels);
        onOpenChange(false);
    };

    const selectedCount = selectedModels.filter(m => m.is_selected).length;

    if (!modelData) return null;

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="max-w-4xl">
                <DialogHeader>
                    <DialogTitle>选择模型</DialogTitle>
                </DialogHeader>
                <TooltipProvider>
                
                <div className="space-y-4">
                    {/* 操作按钮 */}
                    <div className="flex items-center justify-between">
                        <div className="flex gap-2">
                            <Button 
                                variant="outline" 
                                size="sm" 
                                onClick={handleSelectAll}
                            >
                                全选
                            </Button>
                            <Button 
                                variant="outline" 
                                size="sm" 
                                onClick={handleDeselectAll}
                            >
                                取消全选
                            </Button>
                        </div>
                        <div className="text-sm text-muted-foreground">
                            已选择 {selectedCount} / {modelData.available_models.length} 个模型
                        </div>
                    </div>

                    {/* 缺失模型提醒 */}
                    {modelData.missing_models.length > 0 && (
                        <div className="p-3 bg-orange-50/80 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg">
                            <div className="flex items-center gap-2 text-orange-800 dark:text-orange-200 font-medium mb-2">
                                <AlertTriangle className="h-4 w-4" />
                                以下模型未找到，将自动删除
                            </div>
                            <div className="flex flex-wrap gap-1">
                                {modelData.missing_models.map((model, index) => (
                                    <Badge key={index} variant="outline" className="text-orange-700 dark:text-orange-300 border-orange-300 dark:border-orange-700">
                                        {model}
                                    </Badge>
                                ))}
                            </div>
                        </div>
                    )}

                    <Separator />

                    {/* 模型列表 */}
                    <ScrollArea className="h-96">
                        <div className="grid grid-cols-2 gap-4">
                            {selectedModels.map((model) => (
                                <div 
                                    key={model.code} 
                                    className="flex items-center space-x-3 p-3 border rounded-lg hover:bg-muted cursor-pointer"
                                    onClick={() => handleModelToggle(model.code, !model.is_selected)}
                                >
                                    <Checkbox
                                        checked={model.is_selected}
                                        onCheckedChange={(checked) => 
                                            handleModelToggle(model.code, checked as boolean)
                                        }
                                        onClick={(e) => e.stopPropagation()}
                                    />
                                    <div className="flex-1 min-w-0">
                                        <div className="flex items-center gap-2">
                                            <Tooltip>
                                                <TooltipTrigger asChild>
                                                    <div className="font-medium truncate">
                                                        {model.name}
                                                    </div>
                                                </TooltipTrigger>
                                                <TooltipContent>
                                                    <p>{model.name}</p>
                                                </TooltipContent>
                                            </Tooltip>
                                            <div className="flex gap-1">
                                                {model.vision_support && (
                                                    <Eye className="h-3 w-3 text-blue-500" />
                                                )}
                                                {model.audio_support && (
                                                    <Mic className="h-3 w-3 text-green-500" />
                                                )}
                                                {model.video_support && (
                                                    <Video className="h-3 w-3 text-purple-500" />
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </ScrollArea>
                </div>
                </TooltipProvider>

                <DialogFooter>
                    <Button variant="outline" onClick={() => onOpenChange(false)}>
                        取消
                    </Button>
                    <Button onClick={handleConfirm} disabled={loading}>
                        {loading ? '更新中...' : '确认更新'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default ModelSelectionDialog;