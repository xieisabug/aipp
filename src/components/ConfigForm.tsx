import React, { useState, useEffect, useRef, useMemo } from "react";
import { Controller, SubmitHandler, UseFormReturn } from "react-hook-form";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "./ui/select";
import IconButton from "./IconButton";
import { Copy, Delete, Edit, CircleHelp } from "lucide-react";
import "../styles/ConfigForm.css";
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from "./ui/card";
import { Form, FormControl, FormItem, FormLabel, FormMessage } from "./ui/form";
import { Button } from "./ui/button";
import { Textarea } from "./ui/textarea";
import { RadioGroup, RadioGroupItem } from "./ui/radio-group";
import { Input } from "./ui/input";
import { Checkbox } from "./ui/checkbox";
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "./ui/tooltip";

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
        | "button";
    label: string;
    className?: string;
    options?: { value: string; label: string; tooltip?: string }[];
    value?: string | boolean;
    tooltip?: string;
    onChange?: (value: string | boolean) => void;
    onBlur?: (value: string | boolean) => void;
    customRender?: (fieldRenderData: any) => React.ReactNode;
    onClick?: (assistantConfigApi?: AssistantConfigApi) => void;
}

interface ConfigFormProps {
    title: string;
    description?: string;
    config: Array<{ key: string; config: ConfigField }>;
    classNames?: string;
    enableExpand?: boolean;
    defaultExpanded?: boolean;
    useFormReturn: UseFormReturn<any, any, undefined>;
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
                content.removeEventListener(
                    "transitionend",
                    handleTransitionEnd,
                );
                content.removeEventListener(
                    "transitionstart",
                    handleTransitionStart,
                );
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

    const CustomFormField = React.memo(
        ({ field, name }: { field: ConfigField; name: string }) => {
            const renderField = (fieldRenderData: any) => {
                switch (field.type) {
                    case "select":
                        return (
                            <Select
                                value={fieldRenderData.value}
                                onValueChange={fieldRenderData.onChange}
                            >
                                <SelectTrigger>
                                    <SelectValue placeholder={field.label} />
                                </SelectTrigger>
                                <SelectContent>
                                    {field.options?.map((option) => (
                                        <SelectItem
                                            key={option.value}
                                            value={option.value}
                                        >
                                            {option.label}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        );
                    case "textarea":
                        return (
                            <Textarea
                                className={field.className}
                                {...fieldRenderData}
                            />
                        );
                    case "input":
                    case "password":
                        return (
                            <Input
                                className={field.className}
                                type={
                                    field.type === "password"
                                        ? "password"
                                        : "text"
                                }
                                {...fieldRenderData}
                            />
                        );
                    case "checkbox":
                        return (
                            <Checkbox
                                className={field.className}
                                checked={fieldRenderData.value}
                                onCheckedChange={fieldRenderData.onChange}
                            />
                        );
                    case "radio":
                        return (
                            <RadioGroup
                                className={field.className}
                                value={fieldRenderData.value}
                                onValueChange={fieldRenderData.onChange}
                            >
                                {field.options?.map((option) => (
                                    <FormItem
                                        className="flex items-center space-x-2"
                                        key={option.value}
                                    >
                                        <FormControl>
                                            <RadioGroupItem
                                                value={option.value}
                                                id={option.value}
                                            />
                                        </FormControl>
                                        <FormLabel
                                            className="font-normal"
                                            htmlFor={option.value}
                                        >
                                            {option.label}
                                        </FormLabel>
                                        {option.tooltip && (
                                            <span
                                                className="tooltip-trigger"
                                                title={field.tooltip}
                                            >
                                                ?
                                            </span>
                                        )}
                                    </FormItem>
                                ))}
                            </RadioGroup>
                        );
                    case "static":
                        return (
                            <div className={field.className}>{field.value}</div>
                        );
                    case "custom":
                        const customElement = useMemo(() => {
                            return field.customRender
                                ? field.customRender(fieldRenderData)
                                : null;
                        }, [field.customRender, fieldRenderData]);
                        return customElement;
                    case "button":
                        return (
                            <Button
                                type="button"
                                className={field.className}
                                onClick={() => {
                                    field.onClick &&
                                        field.onClick(assistantConfigApi);
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
                        <FormItem className="space-y-2 mb-5">
                            <FormLabel className="flex items-center font-bold text-sm ">
                                {field.label}
                                {field.tooltip && (
                                    <TooltipProvider>
                                        <Tooltip>
                                            <TooltipTrigger>
                                                <CircleHelp
                                                    size={16}
                                                    color="black"
                                                    className="ml-2"
                                                />
                                            </TooltipTrigger>
                                            <TooltipContent>
                                                {field.tooltip}
                                            </TooltipContent>
                                        </Tooltip>
                                    </TooltipProvider>
                                )}
                            </FormLabel>
                            <FormControl>
                                {renderField(fieldRenderData)}
                            </FormControl>
                            <FormMessage />
                        </FormItem>
                    )}
                />
            );
        },
    );

    const renderContent = () => {
        switch (layout) {
            case "prompt":
                return (
                    <div className="assistant-config-grid">
                        <div className="assistant-config-properties">
                            {config
                                .filter((item) => item.key !== "prompt")
                                .map((item) => (
                                    <CustomFormField
                                        name={item.key}
                                        field={item.config}
                                        key={item.key}
                                    />
                                ))}
                        </div>
                        {config.find((item) => item.key === "prompt") && (
                            <div className="assistant-config-prompts">
                                <CustomFormField
                                    name="prompt"
                                    field={
                                        config.find(
                                            (item) => item.key === "prompt",
                                        )!.config
                                    }
                                />
                            </div>
                        )}
                    </div>
                );
            case "provider":
                return (
                    <div className="provider-config-item-form">
                        <div className="provider-config-item-form-property-container">
                            {config.map((item) => (
                                <CustomFormField
                                    name={item.key}
                                    field={item.config}
                                    key={item.key}
                                />
                            ))}
                        </div>
                        {config.find((item) => item.key === "modelList") && (
                            <div className="provider-config-item-form-model-list-container">
                                <CustomFormField
                                    name="model_list"
                                    field={
                                        config.find(
                                            (item) => item.key === "modelList",
                                        )!.config
                                    }
                                />
                            </div>
                        )}
                    </div>
                );
            default:
                return (
                    <div>
                        {config.map((item) => (
                            <CustomFormField
                                name={item.key}
                                field={item.config}
                                key={item.key}
                            />
                        ))}
                    </div>
                );
        }
    };

    return (
        <Card
            className={
                classNames
                    ? classNames + " config-window-container"
                    : "config-window-container"
            }
        >
            <CardHeader
                onClick={toggleExpand}
                className="flex flex-row items-center cursor-pointer"
            >
                <div className="grid gap-2">
                    <CardTitle>{title}</CardTitle>
                    <CardDescription>{description}</CardDescription>
                </div>
                <div className="flex items-center ml-auto gap-1">
                    {onCopy && (
                        <IconButton
                            icon={<Copy color="black" size={16} />}
                            onClick={onCopy}
                        />
                    )}
                    {onDelete && (
                        <IconButton
                            icon={<Delete color="black" size={16} />}
                            onClick={onDelete}
                        />
                    )}
                    {onEdit && (
                        <IconButton
                            icon={<Edit color="black" size={16} />}
                            onClick={onEdit}
                        />
                    )}
                    {extraButtons}
                </div>
            </CardHeader>

            <CardContent
                ref={contentRef}
                className={`config-window-content ${isExpanded ? "expanded" : ""}`}
            >
                <Form {...useFormReturn}>
                    {renderContent()}
                    {onSave && (
                        <div>
                            <Button onClick={onSave}>保存</Button>
                        </div>
                    )}
                </Form>
            </CardContent>
        </Card>
    );
};

export default React.memo(ConfigForm);
