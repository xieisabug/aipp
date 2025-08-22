import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { useTheme } from "@/hooks/useTheme";
import { Search, Star, Download, Shield, ChevronLeft, ChevronRight } from "lucide-react";

interface Plugin {
    id: string;
    name: string;
    icon: string;
    description: string;
    install_count: number;
    is_official: boolean;
    rating: number;
    version: string;
    author: string;
    tags: string[];
    detailed_description?: string;
    screenshots?: string[];
    reviews?: Review[];
    installed?: boolean;
}

interface Review {
    id: string;
    user: string;
    rating: number;
    comment: string;
    date: string;
}

export default function PluginStoreWindow() {
    useTheme();

    const [plugins] = useState<Plugin[]>([
        {
            id: "1",
            name: "代码生成助手",
            icon: "🔧",
            description: "智能生成高质量代码片段",
            install_count: 15420,
            is_official: true,
            rating: 4.8,
            version: "1.2.0",
            author: "AIPP Team",
            tags: ["代码", "生成", "AI"],
            detailed_description: "这是一个强大的代码生成工具，能够根据自然语言描述生成高质量的代码片段。支持多种编程语言，包括 Python、JavaScript、Rust 等。",
            installed: false,
        },
        {
            id: "2",
            name: "文档翻译器",
            icon: "🌐",
            description: "多语言文档智能翻译",
            install_count: 8960,
            is_official: false,
            rating: 4.5,
            version: "0.9.3",
            author: "Community Dev",
            tags: ["翻译", "文档", "多语言"],
            detailed_description: "支持 50+ 种语言的文档翻译工具，保持原有格式和结构。",
            installed: true,
        },
        {
            id: "3",
            name: "API 测试工具",
            icon: "⚡",
            description: "快速测试和调试 API 接口",
            install_count: 12350,
            is_official: true,
            rating: 4.7,
            version: "2.1.0",
            author: "AIPP Team",
            tags: ["API", "测试", "调试"],
            detailed_description: "专业的 API 测试工具，支持 REST、GraphQL 等多种协议。",
            installed: false,
        },
    ]);

    const [searchQuery, setSearchQuery] = useState("");
    const [currentPage, setCurrentPage] = useState(1);
    const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
    const [showDetailDialog, setShowDetailDialog] = useState(false);
    const [activeTab, setActiveTab] = useState("store");

    const itemsPerPage = 12;

    const filteredPlugins = plugins.filter(plugin => {
        const matchesSearch = plugin.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                            plugin.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                            plugin.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
        
        if (activeTab === "installed") {
            return matchesSearch && plugin.installed;
        }
        return matchesSearch;
    });

    const totalPages = Math.ceil(filteredPlugins.length / itemsPerPage);
    const currentPlugins = filteredPlugins.slice(
        (currentPage - 1) * itemsPerPage,
        currentPage * itemsPerPage
    );

    const handlePluginClick = (plugin: Plugin) => {
        setSelectedPlugin(plugin);
        setShowDetailDialog(true);
    };

    const renderStars = (rating: number) => {
        return Array.from({ length: 5 }, (_, i) => (
            <Star
                key={i}
                className={`w-4 h-4 ${
                    i < Math.floor(rating) 
                        ? "fill-yellow-400 text-yellow-400" 
                        : "text-gray-300"
                }`}
            />
        ));
    };

    return (
        <div className="flex flex-col h-screen bg-background p-6">
            <div className="flex flex-col gap-4 mb-6">
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-2xl font-bold">插件商店</h1>
                        <p className="text-muted-foreground">
                            发现和管理您的插件
                        </p>
                    </div>
                </div>

                <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
                    <div className="flex items-center gap-4">
                        <TabsList>
                            <TabsTrigger value="store">商店</TabsTrigger>
                            <TabsTrigger value="installed">已安装</TabsTrigger>
                        </TabsList>

                        <div className="relative flex-1 max-w-md">
                            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
                            <Input
                                placeholder="搜索插件..."
                                value={searchQuery}
                                onChange={(e) => {
                                    setSearchQuery(e.target.value);
                                    setCurrentPage(1);
                                }}
                                className="pl-10"
                            />
                        </div>
                    </div>

                    <TabsContent value="store" className="mt-4">
                        <div className="flex-1 flex flex-col">
                            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 flex-1">
                                {currentPlugins.map((plugin) => (
                                    <div
                                        key={plugin.id}
                                        className="border rounded-lg p-4 cursor-pointer hover:shadow-md transition-shadow bg-card"
                                        onClick={() => handlePluginClick(plugin)}
                                    >
                                        <div className="flex items-start gap-3 mb-3">
                                            <div className="text-2xl">{plugin.icon}</div>
                                            <div className="flex-1 min-w-0">
                                                <div className="flex items-center gap-2 mb-1">
                                                    <h3 className="font-semibold truncate">{plugin.name}</h3>
                                                    {plugin.is_official && (
                                                        <Shield className="w-4 h-4 text-blue-500" />
                                                    )}
                                                </div>
                                                <p className="text-sm text-muted-foreground mb-2 line-clamp-2">
                                                    {plugin.description}
                                                </p>
                                            </div>
                                        </div>

                                        <div className="flex items-center justify-between mb-2">
                                            <div className="flex items-center gap-1">
                                                {renderStars(plugin.rating)}
                                                <span className="text-sm text-muted-foreground ml-1">
                                                    {plugin.rating.toFixed(1)}
                                                </span>
                                            </div>
                                            <div className="flex items-center gap-1 text-sm text-muted-foreground">
                                                <Download className="w-4 h-4" />
                                                {plugin.install_count.toLocaleString()}
                                            </div>
                                        </div>

                                        <div className="flex items-center justify-between">
                                            <div className="flex flex-wrap gap-1">
                                                {plugin.tags.slice(0, 2).map((tag) => (
                                                    <Badge key={tag} variant="secondary" className="text-xs">
                                                        {tag}
                                                    </Badge>
                                                ))}
                                                {plugin.tags.length > 2 && (
                                                    <Badge variant="secondary" className="text-xs">
                                                        +{plugin.tags.length - 2}
                                                    </Badge>
                                                )}
                                            </div>
                                            <Button
                                                size="sm"
                                                variant={plugin.installed ? "secondary" : "default"}
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                }}
                                            >
                                                {plugin.installed ? "已安装" : "安装"}
                                            </Button>
                                        </div>
                                    </div>
                                ))}
                            </div>

                            {totalPages > 1 && (
                                <div className="flex items-center justify-center gap-2 mt-6">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => setCurrentPage(prev => Math.max(1, prev - 1))}
                                        disabled={currentPage === 1}
                                    >
                                        <ChevronLeft className="w-4 h-4" />
                                    </Button>
                                    <span className="text-sm text-muted-foreground">
                                        第 {currentPage} 页，共 {totalPages} 页
                                    </span>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => setCurrentPage(prev => Math.min(totalPages, prev + 1))}
                                        disabled={currentPage === totalPages}
                                    >
                                        <ChevronRight className="w-4 h-4" />
                                    </Button>
                                </div>
                            )}
                        </div>
                    </TabsContent>

                    <TabsContent value="installed" className="mt-4">
                        <div className="flex-1 flex flex-col">
                            {currentPlugins.length === 0 ? (
                                <div className="flex items-center justify-center h-64">
                                    <div className="text-center">
                                        <div className="text-4xl mb-4">📦</div>
                                        <p className="text-muted-foreground">
                                            {searchQuery ? "没有找到匹配的已安装插件" : "还没有安装任何插件"}
                                        </p>
                                    </div>
                                </div>
                            ) : (
                                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 flex-1">
                                    {currentPlugins.map((plugin) => (
                                        <div
                                            key={plugin.id}
                                            className="border rounded-lg p-4 cursor-pointer hover:shadow-md transition-shadow bg-card"
                                            onClick={() => handlePluginClick(plugin)}
                                        >
                                            <div className="flex items-start gap-3 mb-3">
                                                <div className="text-2xl">{plugin.icon}</div>
                                                <div className="flex-1 min-w-0">
                                                    <div className="flex items-center gap-2 mb-1">
                                                        <h3 className="font-semibold truncate">{plugin.name}</h3>
                                                        {plugin.is_official && (
                                                            <Shield className="w-4 h-4 text-blue-500" />
                                                        )}
                                                    </div>
                                                    <p className="text-sm text-muted-foreground mb-2 line-clamp-2">
                                                        {plugin.description}
                                                    </p>
                                                </div>
                                            </div>

                                            <div className="flex items-center justify-between mb-2">
                                                <div className="flex items-center gap-1">
                                                    {renderStars(plugin.rating)}
                                                    <span className="text-sm text-muted-foreground ml-1">
                                                        {plugin.rating.toFixed(1)}
                                                    </span>
                                                </div>
                                                <Badge variant="secondary" className="text-xs">
                                                    v{plugin.version}
                                                </Badge>
                                            </div>

                                            <div className="flex items-center justify-between">
                                                <div className="flex flex-wrap gap-1">
                                                    {plugin.tags.slice(0, 2).map((tag) => (
                                                        <Badge key={tag} variant="secondary" className="text-xs">
                                                            {tag}
                                                        </Badge>
                                                    ))}
                                                </div>
                                                <Button
                                                    size="sm"
                                                    variant="destructive"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                    }}
                                                >
                                                    卸载
                                                </Button>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    </TabsContent>
                </Tabs>
            </div>

            <Dialog open={showDetailDialog} onOpenChange={setShowDetailDialog}>
                <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
                    {selectedPlugin && (
                        <>
                            <DialogHeader>
                                <div className="flex items-start gap-4">
                                    <div className="text-4xl">{selectedPlugin.icon}</div>
                                    <div className="flex-1">
                                        <div className="flex items-center gap-2 mb-2">
                                            <DialogTitle className="text-xl">{selectedPlugin.name}</DialogTitle>
                                            {selectedPlugin.is_official && (
                                                <Shield className="w-5 h-5 text-blue-500" />
                                            )}
                                        </div>
                                        <div className="flex items-center gap-4 text-sm text-muted-foreground mb-2">
                                            <span>作者: {selectedPlugin.author}</span>
                                            <span>版本: {selectedPlugin.version}</span>
                                        </div>
                                        <div className="flex items-center gap-4">
                                            <div className="flex items-center gap-1">
                                                {renderStars(selectedPlugin.rating)}
                                                <span className="text-sm text-muted-foreground ml-1">
                                                    {selectedPlugin.rating.toFixed(1)}
                                                </span>
                                            </div>
                                            <div className="flex items-center gap-1 text-sm text-muted-foreground">
                                                <Download className="w-4 h-4" />
                                                {selectedPlugin.install_count.toLocaleString()} 次安装
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </DialogHeader>

                            <div className="space-y-4">
                                <div>
                                    <h4 className="font-semibold mb-2">简介</h4>
                                    <p className="text-sm text-muted-foreground">
                                        {selectedPlugin.description}
                                    </p>
                                </div>

                                {selectedPlugin.detailed_description && (
                                    <div>
                                        <h4 className="font-semibold mb-2">详细说明</h4>
                                        <p className="text-sm text-muted-foreground">
                                            {selectedPlugin.detailed_description}
                                        </p>
                                    </div>
                                )}

                                <div>
                                    <h4 className="font-semibold mb-2">标签</h4>
                                    <div className="flex flex-wrap gap-2">
                                        {selectedPlugin.tags.map((tag) => (
                                            <Badge key={tag} variant="secondary">
                                                {tag}
                                            </Badge>
                                        ))}
                                    </div>
                                </div>

                                <div>
                                    <h4 className="font-semibold mb-2">用户评价</h4>
                                    <div className="text-center py-8 text-muted-foreground">
                                        暂无评价数据
                                    </div>
                                </div>
                            </div>

                            <DialogFooter>
                                <Button variant="outline" onClick={() => setShowDetailDialog(false)}>
                                    关闭
                                </Button>
                                <Button variant={selectedPlugin.installed ? "destructive" : "default"}>
                                    {selectedPlugin.installed ? "卸载" : "安装"}
                                </Button>
                            </DialogFooter>
                        </>
                    )}
                </DialogContent>
            </Dialog>
        </div>
    );
}
