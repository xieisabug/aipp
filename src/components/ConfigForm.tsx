import React, { useState, useEffect, useRef, useMemo } from "react";
import { Controller, SubmitHandler, UseFormReturn } from "react-hook-form";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Copy, Trash2, CircleHelp, ChevronDown, ChevronRight, Edit3 } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Form, FormControl, FormItem, FormLabel, FormMessage } from "./ui/form";
import { Button } from "./ui/button";
import { Textarea } from "./ui/textarea";
import { RadioGroup, RadioGroupItem } from "./ui/radio-group";
import { Input } from "./ui/input";
import { Checkbox } from "./ui/checkbox";
import { Switch } from "./ui/switch";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";
import { useModels } from "../hooks/useModels";
import { useMcpServers } from "../hooks/useMcpServers";

interface ConfigField {
    type:
        | "select"
        | "textarea"
        | "input"
        | "password"
        | "checkbox"
        | "radio"
        | "static"
        | "custom"
        | "button"
        | "switch"
        | "model-select"
        | "mcp-select";
    label: string;
    className?: string;
    options?: { value: string; label: string; tooltip?: string }[];
    value?: string | boolean;
    tooltip?: string;
    onChange?: (value: string | boolean) => void;
    onBlur?: (value: string | boolean) => void;
    customRender?: (fieldRenderData: any) => React.ReactNode;
    onClick?: (assistantConfigApi?: AssistantConfigApi) => void;
    disabled?: boolean;
    hidden?: boolean;
}

interface ConfigFormProps {
    title: string;
    description?: string;
    config: Array<{ key: string; config: ConfigField }>;
    classNames?: string;
    enableExpand?: boolean;
    defaultExpanded?: boolean;
    useFormReturn: UseFormReturn<any, any, any>;
    assistantConfigApi?: AssistantConfigApi;
    layout?: "default" | "prompt" | "provider";
    onSave?: SubmitHandler<any>;
    onCopy?: () => void;
    onDelete?: () => void;
    onEdit?: () => void;
    extraButtons?: React.ReactNode;
}

const ConfigForm: React.FC<ConfigFormProps> = ({
    title,
    description,
    config,
    classNames,
    enableExpand = false,
    defaultExpanded = true,
    layout = "default",
    useFormReturn,
    assistantConfigApi,
    onSave,
    onCopy,
    onDelete,
    onEdit,
    extraButtons,
}) => {
    const [isExpanded, setIsExpanded] = useState<boolean>(defaultExpanded);
    const contentRef = useRef<HTMLDivElement>(null);

    // 检查是否需要获取模型数据
    const hasModelSelect = useMemo(() => {
        return config.some(item => item.config.type === "model-select");
    }, [config]);

    // 检查是否需要获取MCP服务器数据
    const hasMcpSelect = useMemo(() => {
        return config.some(item => item.config.type === "mcp-select");
    }, [config]);

    // 条件获取模型数据用于 model-select 类型
    const { models, loading: modelsLoading, error: modelsError } = useModels(hasModelSelect);
    
    // 条件获取MCP服务器数据用于 mcp-select 类型
    const { mcpServers, loading: mcpServersLoading, error: mcpServersError } = useMcpServers(hasMcpSelect);

    const toggleExpand = () => {
        if (enableExpand) {
            setIsExpanded(!isExpanded);
        }
    };

    useEffect(() => {
        const content = contentRef.current;
        if (content) {
            const handleTransitionEnd = () => {
                if (isExpanded) {
                    content.style.overflow = "visible";
                }
            };
            const handleTransitionStart = () => {
                if (!isExpanded) {
                    content.style.overflow = "hidden";
                }
            };
            content.addEventListener("transitionend", handleTransitionEnd);
            content.addEventListener("transitionstart", handleTransitionStart);

            return () => {
                content.removeEventListener("transitionend", handleTransitionEnd);
                content.removeEventListener("transitionstart", handleTransitionStart);
            };
        }
    }, [isExpanded]);

    useEffect(() => {
        const content = contentRef.current;

        if (content) {
            if (isExpanded) {
                content.style.overflow = "visible";
            } else {
                content.style.overflow = "hidden";
            }
        }
    }, []);

    const CustomFormField = React.memo(({ field, name }: { field: ConfigField; name: string }) => {
        // 生成模型选项用于 model-select 类型
        const modelOptions = useMemo(() => {
            if (field.type === "model-select" && models) {
                return models.map((model) => ({
                    value: `${model.code}%%${model.llm_provider_id}`,
                    label: model.name,
                }));
            }
            return [];
        }, [field.type, models]);
        
        // 生成MCP服务器选项用于 mcp-select 类型
        const mcpOptions = useMemo(() => {
            if (field.type === "mcp-select" && mcpServers) {
                return mcpServers.map((server) => ({
                    value: server.id,
                    label: server.name,
                }));
            }
            return [];
        }, [field.type, mcpServers]);

        const renderField = (fieldRenderData: any) => {
            switch (field.type) {
                case "select":
                    return (
                        <Select
                            disabled={field.disabled}
                            value={fieldRenderData.value}
                            onValueChange={fieldRenderData.onChange}
                        >
                            <SelectTrigger className="w-full max-w-full focus:ring-ring/20 focus:border-ring overflow-hidden">
                                <SelectValue placeholder={field.label} />
                            </SelectTrigger>
                            <SelectContent>
                                {field.options?.map((option) => (
                                    <SelectItem key={option.value} value={option.value}>
                                        {option.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    );
                case "model-select":
                    return (
                        <Select
                            disabled={field.disabled || modelsLoading}
                            value={fieldRenderData.value}
                            onValueChange={fieldRenderData.onChange}
                        >
                            <SelectTrigger className="w-full max-w-full focus:ring-ring/20 focus:border-ring overflow-hidden">
                                <SelectValue
                                    placeholder={modelsLoading ? "加载中..." : modelsError ? "加载失败" : "选择模型"}
                                />
                            </SelectTrigger>
                            <SelectContent>
                                {modelOptions.map((option) => (
                                    <SelectItem key={option.value} value={option.value}>
                                        {option.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    );
                case "mcp-select":
                    return (
                        <Select
                            disabled={field.disabled || mcpServersLoading}
                            value={fieldRenderData.value}
                            onValueChange={fieldRenderData.onChange}
                        >
                            <SelectTrigger className="w-full max-w-full focus:ring-ring/20 focus:border-ring overflow-hidden">
                                <SelectValue
                                    placeholder={mcpServersLoading ? "加载中..." : mcpServersError ? "加载失败" : "选择MCP服务器"}
                                />
                            </SelectTrigger>
                            <SelectContent>
                                {mcpOptions.map((option) => (
                                    <SelectItem key={option.value} value={option.value? option.value.toString() : ""}>
                                        {option.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    );
                case "textarea":
                    return (
                        <Textarea
                            className={`focus:ring-ring/20 focus:border-ring ${field.className || ""}`}
                            disabled={field.disabled}
                            {...fieldRenderData}
                        />
                    );
                case "input":
                case "password":
                    return (
                        <Input
                            className={`focus:ring-ring/20 focus:border-ring ${field.className || ""}`}
                            type={field.type === "password" ? "password" : "text"}
                            disabled={field.disabled}
                            {...fieldRenderData}
                        />
                    );
                case "checkbox":
                    return (
                        <Checkbox
                            className={`focus:ring-ring/20 ${field.className || ""}`}
                            disabled={field.disabled}
                            checked={fieldRenderData.value}
                            onCheckedChange={fieldRenderData.onChange}
                        />
                    );
                case "switch":
                    return (
                        <Switch
                            className={field.className}
                            disabled={field.disabled}
                            checked={
                                fieldRenderData.value === true ||
                                fieldRenderData.value === "true" ||
                                fieldRenderData.value === 1 ||
                                fieldRenderData.value === "1"
                            }
                            onCheckedChange={fieldRenderData.onChange}
                        />
                    );
                case "radio":
                    return (
                        <RadioGroup
                            className={field.className}
                            value={fieldRenderData.value}
                            onValueChange={fieldRenderData.onChange}
                            disabled={field.disabled}
                        >
                            {field.options?.map((option) => (
                                <FormItem className="flex items-center space-x-2" key={option.value}>
                                    <RadioGroupItem
                                        value={option.value}
                                        id={option.value}
                                        className="focus:ring-ring/20"
                                    />
                                    <label
                                        htmlFor={option.value}
                                        className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-foreground"
                                    >
                                        {option.label}
                                    </label>
                                    {option.tooltip && (
                                        <TooltipProvider>
                                            <Tooltip>
                                                <TooltipTrigger>
                                                    <CircleHelp size={16} className="text-muted-foreground" />
                                                </TooltipTrigger>
                                                <TooltipContent>{option.tooltip}</TooltipContent>
                                            </Tooltip>
                                        </TooltipProvider>
                                    )}
                                </FormItem>
                            ))}
                        </RadioGroup>
                    );
                case "static":
                    return (
                        <div
                            className={`text-sm text-muted-foreground px-3 py-2 bg-muted rounded-md break-words whitespace-pre-wrap ${
                                field.className || ""
                            }`}
                        >
                            {field.value}
                        </div>
                    );
                case "custom":
                    const customElement = useMemo(() => {
                        return field.customRender ? field.customRender(fieldRenderData) : null;
                    }, [field.customRender, fieldRenderData]);
                    return customElement;
                case "button":
                    return (
                        <Button
                            type="button"
                            variant="outline"
                            className={`hover:bg-muted hover:border-border ${field.className || ""}`}
                            disabled={field.disabled}
                            onClick={() => {
                                field.onClick && field.onClick(assistantConfigApi);
                            }}
                        >
                            {field.value as string}
                        </Button>
                    );
                default:
                    return null;
            }
        };

        return (
            <Controller
                control={useFormReturn.control}
                name={name}
                render={({ field: fieldRenderData }: { field: any }) => (
                    <FormItem className={`space-y-3 mb-6 ${field.hidden ? "hidden" : ""}`}>
                        <FormLabel className="flex items-center font-semibold text-sm text-foreground">
                            {field.label}
                            {field.tooltip && (
                                <TooltipProvider>
                                    <Tooltip>
                                        <TooltipTrigger className="ml-2">
                                            <CircleHelp
                                                size={16}
                                                className="text-muted-foreground hover:text-foreground transition-colors"
                                            />
                                        </TooltipTrigger>
                                        <TooltipContent>{field.tooltip}</TooltipContent>
                                    </Tooltip>
                                </TooltipProvider>
                            )}
                        </FormLabel>
                        <FormControl>{renderField(fieldRenderData)}</FormControl>
                        <FormMessage />
                    </FormItem>
                )}
            />
        );
    });

    const renderContent = () => {
        switch (layout) {
            case "prompt":
                return (
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
                        <div className="space-y-6">
                            {config
                                .filter((item) => item.key !== "prompt")
                                .map((item) => (
                                    <CustomFormField name={item.key} field={item.config} key={item.key} />
                                ))}
                        </div>
                        {config.find((item) => item.key === "prompt") && (
                            <div className="space-y-6">
                                <CustomFormField
                                    name="prompt"
                                    field={config.find((item) => item.key === "prompt")!.config}
                                />
                            </div>
                        )}
                    </div>
                );
            case "provider":
                return (
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
                        <div className="space-y-6">
                            {config.map((item) => (
                                <CustomFormField name={item.key} field={item.config} key={item.key} />
                            ))}
                        </div>
                        {config.find((item) => item.key === "modelList") && (
                            <div className="space-y-6">
                                <CustomFormField
                                    name="model_list"
                                    field={config.find((item) => item.key === "modelList")!.config}
                                />
                            </div>
                        )}
                    </div>
                );
            default:
                return (
                    <div className="space-y-6">
                        {config.map((item) => (
                            <CustomFormField name={item.key} field={item.config} key={item.key} />
                        ))}
                    </div>
                );
        }
    };

    return (
        <Card className={`shadow-md hover:shadow-lg transition-shadow border-l-4 border-l-primary ${classNames || ""}`}>
            <CardHeader
                onClick={toggleExpand}
                className={`flex flex-row items-center ${
                    enableExpand ? "cursor-pointer hover:bg-muted" : ""
                } transition-colors rounded-t-lg`}
            >
                <div className="flex items-center flex-1 min-w-0">
                    {enableExpand && (
                        <div className="mr-3 text-muted-foreground">
                            {isExpanded ? <ChevronDown className="h-5 w-5" /> : <ChevronRight className="h-5 w-5" />}
                        </div>
                    )}
                    <div className="flex-1 min-w-0">
                        <CardTitle className="text-lg font-semibold text-foreground truncate">{title}</CardTitle>
                        {description && (
                            <CardDescription className="text-sm text-muted-foreground mt-1 truncate">
                                {description}
                            </CardDescription>
                        )}
                    </div>
                </div>
                <div className="flex items-center gap-2 ml-4">
                    {onCopy && (
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={onCopy}
                            className="hover:bg-muted hover:border-border hover:text-foreground"
                        >
                            <Copy className="h-4 w-4 mr-1" />
                        </Button>
                    )}
                    {onEdit && (
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={onEdit}
                            className="hover:bg-muted hover:border-border hover:text-foreground"
                        >
                            <Edit3 className="h-4 w-4 mr-1" />
                        </Button>
                    )}
                    {onDelete && (
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={onDelete}
                            className="hover:bg-destructive/10 hover:border-destructive/30 hover:text-destructive"
                        >
                            <Trash2 className="h-4 w-4 mr-1" />
                        </Button>
                    )}
                    {extraButtons}
                </div>
            </CardHeader>

            <CardContent
                ref={contentRef}
                className={`transition-all duration-300 ease-in-out ${
                    isExpanded ? "max-h-none opacity-100" : "max-h-0 opacity-0 overflow-hidden"
                }`}
            >
                <Form {...useFormReturn}>
                    {renderContent()}
                    {onSave && (
                        <div className="mt-8 pt-4 border-t border-border">
                            <Button onClick={onSave} className="bg-primary hover:bg-primary/90 text-primary-foreground">
                                保存配置
                            </Button>
                        </div>
                    )}
                </Form>
            </CardContent>
        </Card>
    );
};

export default React.memo(ConfigForm);
