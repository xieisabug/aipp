import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
// form removed here; using a dedicated dialog component
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useToast } from '@/hooks/use-toast';
import { useTheme } from '@/hooks/useTheme';
import { formatIconDisplay } from '@/utils/emoji-utils';
import { Search, MoreVertical, Folder, Grid, List } from 'lucide-react';
import EditArtifactDialog from '@/components/EditArtifactDialog';

interface ArtifactCollection {
    id: number;
    name: string;
    icon: string;
    description: string;
    artifact_type: string;
    tags?: string;
    created_time: string;
    last_used_time?: string;
    use_count: number;
}

// (removed old EditArtifactData; replaced by using ArtifactCollection directly)

export default function ArtifactCollectionsWindow() {
    useTheme();

    const [artifacts, setArtifacts] = useState<ArtifactCollection[]>([]);
    const [filteredArtifacts, setFilteredArtifacts] = useState<ArtifactCollection[]>([]);
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedType, setSelectedType] = useState<string>('all');
    const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
    const [isLoading, setIsLoading] = useState(true);
    const [showEditDialog, setShowEditDialog] = useState(false);
    const [editingArtifact, setEditingArtifact] = useState<ArtifactCollection | null>(null);
    const [showDeleteDialog, setShowDeleteDialog] = useState(false);
    const [deletingArtifact, setDeletingArtifact] = useState<ArtifactCollection | null>(null);
    const { toast } = useToast();


    // 加载所有 artifacts
    const loadArtifacts = async () => {
        try {
            setIsLoading(true);
            const result = await invoke<ArtifactCollection[]>('get_artifacts_collection', {
                artifactType: selectedType === 'all' ? null : selectedType
            });
            setArtifacts(result);
            setFilteredArtifacts(result);
        } catch (error) {
            console.error('加载 artifacts 失败:', error);
            toast({
                title: '加载失败',
                description: error as string,
                variant: 'destructive',
            });
        } finally {
            setIsLoading(false);
        }
    };

    // 搜索和过滤
    useEffect(() => {
        let filtered = artifacts;

        // 按类型过滤
        if (selectedType !== 'all') {
            filtered = filtered.filter(artifact => artifact.artifact_type === selectedType);
        }

        // 按搜索词过滤
        if (searchQuery.trim()) {
            const query = searchQuery.toLowerCase();
            filtered = filtered.filter(artifact =>
                artifact.name.toLowerCase().includes(query) ||
                artifact.description.toLowerCase().includes(query) ||
                artifact.tags?.toLowerCase().includes(query)
            );
        }

        setFilteredArtifacts(filtered);
    }, [artifacts, searchQuery, selectedType]);

    // 监听事件
    useEffect(() => {
        const unlisteners: (() => void)[] = [];

        const setupListeners = async () => {
            const artifactDeletedUnlisten = await listen('artifact-collection-updated', () => {
                loadArtifacts();
            });

            unlisteners.push(artifactDeletedUnlisten);
        };

        setupListeners();
        loadArtifacts();

        return () => {
            unlisteners.forEach(unlisten => unlisten());
        };
    }, [selectedType]);

    // 打开 artifact
    const openArtifact = async (artifact: ArtifactCollection) => {
        try {
            await invoke('open_artifact_window', { artifactId: artifact.id });
        } catch (error) {
            console.error('打开 artifact 失败:', error);
            toast({
                title: '打开失败',
                description: error as string,
                variant: 'destructive',
            });
        }
    };

    // 编辑 artifact
    const handleEdit = (artifact: ArtifactCollection) => {
        setEditingArtifact(artifact);
        setShowEditDialog(true);
    };

    // 保存编辑
    // save handled inside the dialog component

    // 删除 artifact
    const handleDelete = (artifact: ArtifactCollection) => {
        setDeletingArtifact(artifact);
        setShowDeleteDialog(true);
    };

    // 确认删除
    const confirmDelete = async () => {
        if (!deletingArtifact) return;

        try {
            await invoke('delete_artifact_collection', { id: deletingArtifact.id });

            toast({
                title: '删除成功',
                description: `已删除 "${deletingArtifact.name}"`,
            });

            setShowDeleteDialog(false);
            setDeletingArtifact(null);
            loadArtifacts();
        } catch (error) {
            console.error('删除失败:', error);
            toast({
                title: '删除失败',
                description: error as string,
                variant: 'destructive',
            });
        }
    };

    // 获取唯一的类型列表
    const artifactTypes = Array.from(new Set(artifacts.map(a => a.artifact_type)));

    return (
        <div className="flex flex-col h-screen bg-background p-6">
            {/* 顶部工具栏 */}
            <div className="flex flex-col gap-4 mb-6">
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-2xl font-bold">Artifacts 合集</h1>
                        <p className="text-muted-foreground">
                            管理您保存的所有 artifacts，共 {artifacts.length} 个项目
                        </p>
                    </div>
                    <div className="flex items-center gap-2">
                        <Button
                            variant={viewMode === 'grid' ? 'default' : 'outline'}
                            size="sm"
                            onClick={() => setViewMode('grid')}
                        >
                            <Grid className="w-4 h-4" />
                        </Button>
                        <Button
                            variant={viewMode === 'list' ? 'default' : 'outline'}
                            size="sm"
                            onClick={() => setViewMode('list')}
                        >
                            <List className="w-4 h-4" />
                        </Button>
                    </div>
                </div>

                {/* 搜索和过滤 */}
                <div className="flex items-center gap-4">
                    <div className="relative flex-1">
                        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
                        <Input
                            placeholder="搜索 artifacts..."
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            className="pl-10"
                        />
                    </div>
                    <select
                        value={selectedType}
                        onChange={(e) => setSelectedType(e.target.value)}
                        className="px-3 py-2 border border-border rounded-md bg-background"
                    >
                        <option value="all">所有类型</option>
                        {artifactTypes.map(type => (
                            <option key={type} value={type}>
                                {type.toUpperCase()}
                            </option>
                        ))}
                    </select>
                </div>
            </div>

            {/* 内容区域 */}
            <div className="flex-1 overflow-auto">
                {isLoading ? (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-center">
                            <Folder className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
                            <p className="text-muted-foreground">加载中...</p>
                        </div>
                    </div>
                ) : filteredArtifacts.length === 0 ? (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-center">
                            <Folder className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
                            <p className="text-muted-foreground">
                                {searchQuery ? '没有找到匹配的 artifacts' : '还没有保存任何 artifacts'}
                            </p>
                        </div>
                    </div>
                ) : (
                    <div className={
                        viewMode === 'grid'
                            ? 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4'
                            : 'space-y-2'
                    }>
                        {filteredArtifacts.map(artifact => (
                            <Card
                                key={artifact.id}
                                className="cursor-pointer hover:bg-accent/50 transition-colors"
                                onClick={() => openArtifact(artifact)}
                            >
                                <CardHeader className="pb-3">
                                    <div className="flex items-start justify-between">
                                        <div className="flex items-center gap-2">
                                            {(() => {
                                                const iconDisplay = formatIconDisplay(artifact.icon);
                                                return iconDisplay.isImage ? (
                                                    <img 
                                                        src={iconDisplay.display} 
                                                        alt={`Icon for ${artifact.name}`} 
                                                        className="w-6 h-6 object-cover rounded border"
                                                    />
                                                ) : (
                                                    <span className="text-2xl">{iconDisplay.display}</span>
                                                );
                                            })()}
                                            <div className="flex-1">
                                                <CardTitle className="text-sm font-medium truncate">
                                                    {artifact.name}
                                                </CardTitle>
                                                <Badge variant="secondary" className="text-xs mt-1">
                                                    {artifact.artifact_type.toUpperCase()}
                                                </Badge>
                                            </div>
                                        </div>
                                        <DropdownMenu>
                                            <DropdownMenuTrigger
                                                asChild
                                                onClick={(e) => e.stopPropagation()}
                                            >
                                                <Button variant="ghost" size="sm">
                                                    <MoreVertical className="w-4 h-4" />
                                                </Button>
                                            </DropdownMenuTrigger>
                                            <DropdownMenuContent align="end">
                                                <DropdownMenuItem onClick={(e) => {
                                                    e.stopPropagation();
                                                    openArtifact(artifact);
                                                }}>
                                                    打开
                                                </DropdownMenuItem>
                                                <DropdownMenuItem onClick={(e) => {
                                                    e.stopPropagation();
                                                    handleEdit(artifact);
                                                }}>
                                                    编辑
                                                </DropdownMenuItem>
                                                <DropdownMenuItem
                                                    className="text-destructive"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        handleDelete(artifact);
                                                    }}
                                                >
                                                    删除
                                                </DropdownMenuItem>
                                            </DropdownMenuContent>
                                        </DropdownMenu>
                                    </div>
                                </CardHeader>
                                <CardContent className="pt-0">
                                    <CardDescription className="text-xs line-clamp-2 mb-2">
                                        {artifact.description || '暂无描述'}
                                    </CardDescription>
                                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                                        <span>使用 {artifact.use_count} 次</span>
                                        <span>{new Date(artifact.created_time).toLocaleDateString()}</span>
                                    </div>
                                </CardContent>
                            </Card>
                        ))}
                    </div>
                )}
            </div>

            {/* 编辑对话框 */}
            <EditArtifactDialog
                isOpen={showEditDialog}
                artifact={editingArtifact}
                onClose={() => {
                    setShowEditDialog(false);
                    setEditingArtifact(null);
                }}
                onUpdated={loadArtifacts}
            />

            {/* 删除确认对话框 */}
            <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>确认删除</DialogTitle>
                        <DialogDescription>
                            确定要删除 "{deletingArtifact?.name}" 吗？此操作无法撤销。
                        </DialogDescription>
                    </DialogHeader>
                    <DialogFooter>
                        <Button variant="outline" onClick={() => setShowDeleteDialog(false)}>
                            取消
                        </Button>
                        <Button variant="destructive" onClick={confirmDelete}>
                            删除
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    );
}