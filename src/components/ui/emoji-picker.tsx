import React, { useState, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useToast } from "@/hooks/use-toast";
import { getEmojiCategories, validateImageFile, resizeImage, formatIconDisplay } from "@/utils/emojiUtils";

interface EmojiPickerProps {
    value: string;
    onChange: (value: string) => void;
    className?: string;
}

const DEFAULT_IMAGE_WIDTH = 128;
const DEFAULT_IMAGE_HEIGHT = 128;
const DEFAULT_IMAGE_QUALITY = 0.8;

export default function EmojiPicker({ value, onChange, className }: EmojiPickerProps) {
    const [open, setOpen] = useState(false);
    const [customImages, setCustomImages] = useState<string[]>([]);
    const fileInputRef = useRef<HTMLInputElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const { toast } = useToast();

    const categories = getEmojiCategories();

    // 点击外部关闭
    React.useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
                setOpen(false);
            }
        };

        if (open) {
            document.addEventListener("mousedown", handleClickOutside);
            return () => {
                document.removeEventListener("mousedown", handleClickOutside);
            };
        }
    }, [open]);

    const handleSelect = (selectedValue: string) => {
        onChange(selectedValue);
        setOpen(false);
    };

    const handleFileUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file) return;

        const validation = validateImageFile(file);
        if (!validation.valid) {
            toast({
                title: "文件格式错误",
                description: validation.error,
                variant: "destructive",
            });
            return;
        }

        try {
            const base64 = await resizeImage(file, DEFAULT_IMAGE_WIDTH, DEFAULT_IMAGE_HEIGHT, DEFAULT_IMAGE_QUALITY);
            setCustomImages((prev) => [...prev, base64]);
            handleSelect(base64);

            toast({
                title: "上传成功",
                description: "图片已成功上传并设置为图标",
            });
        } catch (error) {
            console.error("图片处理失败:", error);
            toast({
                title: "处理失败",
                description: "图片处理失败，请重试",
                variant: "destructive",
            });
        }
    };

    const currentIcon = formatIconDisplay(value);

    return (
        <div ref={containerRef} className={`relative ${className}`} data-emoji-picker="true">
            <Button
                variant="outline"
                className="w-full justify-start gap-2 h-10"
                type="button"
                onClick={() => setOpen(!open)}
                aria-expanded={open}
            >
                {currentIcon.isImage ? (
                    <img src={currentIcon.display} alt="当前图标" className="w-6 h-6 object-cover rounded border" />
                ) : (
                    <span className="text-xl">{currentIcon.display}</span>
                )}
                <span className="text-sm text-muted-foreground">{currentIcon.isImage ? "自定义图片" : "Emoji"}</span>
            </Button>

            {open && (
                <div
                    className="absolute top-full left-0 mt-2 w-80 h-96 bg-popover border rounded-md shadow-lg z-[1001] p-4"
                    style={{
                        pointerEvents: "auto",
                        userSelect: "none",
                        WebkitUserSelect: "none",
                    }}
                    data-emoji-panel="true"
                    onMouseDown={(e) => e.stopPropagation()}
                    onClick={(e) => e.stopPropagation()}
                >
                    <Tabs defaultValue="emojis" className="w-full h-full flex flex-col">
                        <TabsList className="grid w-full grid-cols-2 mb-3 flex-shrink-0">
                            <TabsTrigger value="emojis">Emoji</TabsTrigger>
                            <TabsTrigger value="images">自定义图片</TabsTrigger>
                        </TabsList>

                        <TabsContent value="emojis" className="flex-1 min-h-0 flex flex-col">
                            <ScrollArea className="flex-1 min-h-0">
                                <div className="space-y-3">
                                    {Object.entries(categories).map(([categoryKey, category]) => (
                                        <div key={categoryKey}>
                                            <h4 className="font-medium text-xs text-muted-foreground mb-1">
                                                {category.name}
                                            </h4>
                                            <div className="grid grid-cols-8 gap-1">
                                                {category.emojis.map((emoji, index) => (
                                                    <button
                                                        key={`${categoryKey}-${emoji}-${index}`}
                                                        type="button"
                                                        onMouseDown={(e) => {
                                                            e.preventDefault();
                                                            e.stopPropagation();
                                                            handleSelect(emoji);
                                                        }}
                                                        className={`
                                text-lg p-2 rounded hover:bg-accent transition-colors flex items-center justify-center cursor-pointer
                                ${value === emoji ? "bg-accent ring-2 ring-primary" : ""}
                              `}
                                                    >
                                                        {emoji}
                                                    </button>
                                                ))}
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            </ScrollArea>
                        </TabsContent>

                        <TabsContent value="images" className="flex-1 min-h-0 flex flex-col space-y-2">
                            <div className="flex-shrink-0">
                                <input
                                    type="file"
                                    ref={fileInputRef}
                                    onChange={handleFileUpload}
                                    accept="image/*"
                                    className="hidden"
                                />
                                <Button
                                    onClick={() => fileInputRef.current?.click()}
                                    variant="outline"
                                    size="sm"
                                    type="button"
                                    className="w-full"
                                >
                                    上传图片
                                </Button>
                                <p className="text-xs text-muted-foreground mt-1">
                                    支持 PNG, JPG, GIF, SVG, WebP 格式，最大 5MB
                                </p>
                            </div>

                            <ScrollArea className="flex-1 min-h-0">
                                {customImages.length > 0 ? (
                                    <div className="grid grid-cols-6 gap-2 p-1">
                                        {customImages.map((imageBase64, index) => (
                                            <button
                                                key={index}
                                                type="button"
                                                onMouseDown={(e) => {
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    handleSelect(imageBase64);
                                                }}
                                                className={`
                            w-10 h-10 rounded border hover:ring-2 hover:ring-primary/50 transition-all flex items-center justify-center overflow-hidden cursor-pointer
                            ${value === imageBase64 ? "ring-2 ring-primary" : ""}
                          `}
                                            >
                                                <img
                                                    src={imageBase64}
                                                    alt={`自定义图标 ${index + 1}`}
                                                    className="w-full h-full object-cover"
                                                />
                                            </button>
                                        ))}
                                    </div>
                                ) : (
                                    <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
                                        暂无自定义图片，点击上传按钮添加
                                    </div>
                                )}
                            </ScrollArea>
                        </TabsContent>
                    </Tabs>
                </div>
            )}
        </div>
    );
}
