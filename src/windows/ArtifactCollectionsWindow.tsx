import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import {
    ContextMenu,
    ContextMenuTrigger,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
} from "@/components/ui/context-menu";
import { useToast } from "@/hooks/use-toast";
import { useTheme } from "@/hooks/useTheme";
import { formatIconDisplay } from "@/utils/emojiUtils";
import { Search, Folder } from "lucide-react";
import EditArtifactDialog from "@/components/EditArtifactDialog";

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

export default function ArtifactCollectionsWindow() {
    useTheme();

    const [artifacts, setArtifacts] = useState<ArtifactCollection[]>([]);
    const [filteredArtifacts, setFilteredArtifacts] = useState<ArtifactCollection[]>([]);
    const [searchQuery, setSearchQuery] = useState("");
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
            const result = await invoke<ArtifactCollection[]>("get_artifacts_collection", {
                artifactType: null,
            });
            setArtifacts(result);
            setFilteredArtifacts(result);
        } catch (error) {
            console.error("加载 artifacts 失败:", error);
            toast({
                title: "加载失败",
                description: error as string,
                variant: "destructive",
            });
        } finally {
            setIsLoading(false);
        }
    };

    // 搜索和过滤
    useEffect(() => {
        let filtered = artifacts;

        // 按搜索词过滤
        if (searchQuery.trim()) {
            const query = searchQuery.toLowerCase();
            filtered = filtered.filter(
                (artifact) =>
                    artifact.name.toLowerCase().includes(query) ||
                    artifact.description.toLowerCase().includes(query) ||
                    artifact.tags?.toLowerCase().includes(query)
            );
        }

        setFilteredArtifacts(filtered);
    }, [artifacts, searchQuery]);

    // 监听事件
    useEffect(() => {
        const unlisteners: (() => void)[] = [];

        const setupListeners = async () => {
            const artifactDeletedUnlisten = await listen("artifact-collection-updated", () => {
                loadArtifacts();
            });

            unlisteners.push(artifactDeletedUnlisten);
        };

        setupListeners();
        loadArtifacts();

        return () => {
            unlisteners.forEach((unlisten) => unlisten());
        };
    }, []);

    // 打开 artifact
    const openArtifact = async (artifact: ArtifactCollection) => {
        try {
            await invoke("open_artifact_window", { artifactId: artifact.id });
        } catch (error) {
            console.error("打开 artifact 失败:", error);
            toast({
                title: "打开失败",
                description: error as string,
                variant: "destructive",
            });
        }
    };

    // 编辑 artifact
    const handleEdit = (artifact: ArtifactCollection) => {
        setEditingArtifact(artifact);
        setShowEditDialog(true);
    };

    // 删除 artifact
    const handleDelete = (artifact: ArtifactCollection) => {
        setDeletingArtifact(artifact);
        setShowDeleteDialog(true);
    };

    // 确认删除
    const confirmDelete = async () => {
        if (!deletingArtifact) return;

        try {
            await invoke("delete_artifact_collection", { id: deletingArtifact.id });

            toast({
                title: "删除成功",
                description: `已删除 "${deletingArtifact.name}"`,
            });

            setShowDeleteDialog(false);
            setDeletingArtifact(null);
            loadArtifacts();
        } catch (error) {
            console.error("删除失败:", error);
            toast({
                title: "删除失败",
                description: error as string,
                variant: "destructive",
            });
        }
    };

    return (
        <div className="flex flex-col h-screen bg-background p-6">
            <div className="flex flex-col gap-4 mb-6">
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-2xl font-bold">Artifacts 合集</h1>
                        <p className="text-muted-foreground">
                            管理您保存的所有 artifacts，共 {artifacts.length} 个项目
                        </p>
                    </div>
                </div>

                <div className="relative">
                    <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
                    <Input
                        placeholder="搜索 artifacts..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="pl-10"
                    />
                </div>
            </div>

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
                                {searchQuery ? "没有找到匹配的 artifacts" : "还没有保存任何 artifacts"}
                            </p>
                        </div>
                    </div>
                ) : (
                    <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 2xl:grid-cols-10 gap-3">
                        {filteredArtifacts.map((artifact) => (
                            <ContextMenu key={artifact.id}>
                                <ContextMenuTrigger asChild>
                                    <div
                                        className="flex flex-col items-center p-2 cursor-pointer hover:bg-accent/50 rounded-lg transition-colors group"
                                        onClick={() => openArtifact(artifact)}
                                    >
                                        <div className="mb-2 w-16 h-16">
                                            {(() => {
                                                const iconDisplay = formatIconDisplay(artifact.icon);
                                                return iconDisplay.isImage ? (
                                                    <img
                                                        src={iconDisplay.display}
                                                        alt={`Icon for ${artifact.name}`}
                                                        className="w-16 h-16 object-cover"
                                                    />
                                                ) : (
                                                    <span className="text-6xl">{iconDisplay.display}</span>
                                                );
                                            })()}
                                        </div>

                                        <div className="text-center w-full">
                                            <p className="text-xs font-medium truncate mb-1" title={artifact.name}>
                                                {artifact.name}
                                            </p>
                                            <Badge variant="secondary" className="text-xs">
                                                {artifact.artifact_type}
                                            </Badge>
                                        </div>
                                    </div>
                                </ContextMenuTrigger>

                                <ContextMenuContent>
                                    <ContextMenuItem onClick={() => openArtifact(artifact)}>打开</ContextMenuItem>
                                    <ContextMenuItem onClick={() => handleEdit(artifact)}>编辑</ContextMenuItem>
                                    <ContextMenuSeparator />
                                    <ContextMenuItem variant="destructive" onClick={() => handleDelete(artifact)}>
                                        删除
                                    </ContextMenuItem>
                                    <ContextMenuSeparator />
                                    <ContextMenuItem disabled>使用次数: {artifact.use_count}</ContextMenuItem>
                                    <ContextMenuItem disabled>
                                        创建时间: {new Date(artifact.created_time).toLocaleDateString()}
                                    </ContextMenuItem>
                                    {artifact.description && (
                                        <ContextMenuItem disabled title={artifact.description}>
                                            描述:{" "}
                                            {artifact.description.length > 20
                                                ? artifact.description.substring(0, 20) + "..."
                                                : artifact.description}
                                        </ContextMenuItem>
                                    )}
                                </ContextMenuContent>
                            </ContextMenu>
                        ))}
                    </div>
                )}
            </div>

            <EditArtifactDialog
                isOpen={showEditDialog}
                artifact={editingArtifact}
                onClose={() => {
                    setShowEditDialog(false);
                    setEditingArtifact(null);
                }}
                onUpdated={loadArtifacts}
            />

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
