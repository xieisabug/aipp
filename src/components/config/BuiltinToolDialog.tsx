import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "../ui/dialog";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Input } from "../ui/input";
import { Switch } from "../ui/switch";
import { Card, CardContent } from "../ui/card";

type EnvVarOption = {
    label: string;
    value: string;
};

type BuiltinTemplateEnvVar = {
    key: string;
    label: string;
    required: boolean;
    tip?: string;
    field_type: string; // "text", "select", "boolean", "number"
    default_value?: string;
    placeholder?: string;
    options?: EnvVarOption[];
};

type BuiltinTemplate = {
    id: string;
    name: string;
    description: string;
    command: string;
    transport_type: string;
    required_envs: BuiltinTemplateEnvVar[];
};

interface BuiltinToolDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: () => void;
    editing?: boolean;
    initialName?: string;
    initialDescription?: string;
    initialCommand?: string;
    initialEnvText?: string;
    onEnvChange?: (v: string) => void;
}

const BuiltinToolDialog: React.FC<BuiltinToolDialogProps> = ({
    isOpen,
    onClose,
    onSubmit,
    editing = false,
    initialName,
    initialDescription,
    initialCommand,
    initialEnvText,
    onEnvChange,
}) => {
    const [templates, setTemplates] = useState<BuiltinTemplate[]>([]);
    const [selectedId, setSelectedId] = useState<string>("search");
    const [envValues, setEnvValues] = useState<Record<string, string>>({});
    const [busy, setBusy] = useState(false);

    const selected = useMemo(() => templates.find((t) => t.id === selectedId), [templates, selectedId]);

    // Parse initial envText to envValues
    useEffect(() => {
        if (!initialEnvText) {
            setEnvValues({});
            return;
        }

        const envs: Record<string, string> = {};
        initialEnvText
            .split("\n")
            .map((l) => l.trim())
            .filter(Boolean)
            .forEach((line) => {
                const idx = line.indexOf("=");
                if (idx > 0) {
                    const k = line.slice(0, idx).trim();
                    const v = line.slice(idx + 1).trim();
                    if (k) envs[k] = v;
                }
            });
        setEnvValues(envs);
    }, [initialEnvText]);

    // Set default values when template changes (non-editing mode only)
    useEffect(() => {
        if (editing || !selected) return;

        const defaultValues: Record<string, string> = {};
        selected.required_envs.forEach((env) => {
            if (env.default_value) {
                defaultValues[env.key] = env.default_value;
            }
        });
        setEnvValues(defaultValues);
    }, [selected, editing]);

    useEffect(() => {
        if (!isOpen) return;
        // Always fetch templates, even in editing mode for field definitions
        invoke<BuiltinTemplate[]>("list_aipp_builtin_templates")
            .then(setTemplates)
            .catch(() => {});
    }, [isOpen]);

    // Convert envValues to string format for onEnvChange callback (non-editing mode only)
    useEffect(() => {
        if (onEnvChange && !editing) {
            const envText = Object.entries(envValues)
                .filter(([_, value]) => value !== "")
                .map(([key, value]) => `${key}=${value}`)
                .join("\n");
            onEnvChange(envText);
        }
    }, [envValues, onEnvChange, editing]);

    const handleEnvValueChange = (key: string, value: string) => {
        setEnvValues((prev) => ({
            ...prev,
            [key]: value,
        }));
    };

    const handleSubmit = async () => {
        // In editing mode, we don't need selected template
        if (!editing && !selected) return;

        setBusy(true);
        // Convert envValues to the expected format
        const envs = Object.fromEntries(Object.entries(envValues).filter(([_, value]) => value !== ""));

        try {
            if (!editing) {
                await invoke<number>("add_or_update_aipp_builtin_server", {
                    templateId: selected!.id,
                    name: selected!.name,
                    description: selected!.description,
                    envs,
                });
            } else {
                // In editing mode, just call onSubmit with the parsed environment variables
                // The parent component (MCPConfig) will handle the actual update
            }
            onSubmit();
        } catch (e) {
            // noop; outer page toasts
            console.error(e);
        } finally {
            setBusy(false);
        }
    };

    const renderEnvField = (env: BuiltinTemplateEnvVar) => {
        const value = envValues[env.key] || "";

        const fieldId = `env-${env.key}`;

        switch (env.field_type) {
            case "select":
                return (
                    <div key={env.key} className="space-y-2">
                        <Label htmlFor={fieldId} className="text-sm font-medium">
                            {env.label}
                            {env.required && <span className="text-red-500 ml-1">*</span>}
                        </Label>
                        <Select value={value} onValueChange={(val) => handleEnvValueChange(env.key, val)}>
                            <SelectTrigger>
                                <SelectValue placeholder={env.placeholder || `选择${env.label}`} />
                            </SelectTrigger>
                            <SelectContent>
                                {env.options?.map((option) => (
                                    <SelectItem key={option.value} value={option.value}>
                                        {option.label}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                        {env.tip && <p className="text-xs text-muted-foreground">{env.tip}</p>}
                    </div>
                );

            case "boolean":
                return (
                    <div key={env.key} className="space-y-2">
                        <div className="flex items-center justify-between">
                            <Label htmlFor={fieldId} className="text-sm font-medium">
                                {env.label}
                                {env.required && <span className="text-red-500 ml-1">*</span>}
                            </Label>
                            <Switch
                                id={fieldId}
                                checked={value === "true"}
                                onCheckedChange={(checked) => handleEnvValueChange(env.key, checked ? "true" : "false")}
                            />
                        </div>
                        {env.tip && <p className="text-xs text-muted-foreground">{env.tip}</p>}
                    </div>
                );

            case "number":
                return (
                    <div key={env.key} className="space-y-2">
                        <Label htmlFor={fieldId} className="text-sm font-medium">
                            {env.label}
                            {env.required && <span className="text-red-500 ml-1">*</span>}
                        </Label>
                        <Input
                            id={fieldId}
                            type="number"
                            value={value}
                            placeholder={env.placeholder}
                            onChange={(e) => handleEnvValueChange(env.key, e.target.value)}
                        />
                        {env.tip && <p className="text-xs text-muted-foreground">{env.tip}</p>}
                    </div>
                );

            case "text":
            default:
                return (
                    <div key={env.key} className="space-y-2">
                        <Label htmlFor={fieldId} className="text-sm font-medium">
                            {env.label}
                            {env.required && <span className="text-red-500 ml-1">*</span>}
                        </Label>
                        <Input
                            id={fieldId}
                            type="text"
                            value={value}
                            placeholder={env.placeholder}
                            onChange={(e) => handleEnvValueChange(env.key, e.target.value)}
                        />
                        {env.tip && <p className="text-xs text-muted-foreground">{env.tip}</p>}
                    </div>
                );
        }
    };

    // Get the template to use for rendering fields
    const templateForFields = editing
        ? templates.find((t) => t.id === "search") // Use search template for editing
        : selected;

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="max-w-4xl min-w-xl w-1/2 sm:max-w-none max-h-[80vh] flex flex-col">
                <DialogHeader>
                    <DialogTitle>{editing ? "编辑内置工具环境变量" : "添加内置工具"}</DialogTitle>
                </DialogHeader>
                <div className="space-y-4 overflow-y-auto flex-1 min-h-0 px-4">
                    {/* Template selector; hidden in edit mode */}
                    {!editing && (
                        <div className="space-y-2">
                            <Label>选择内置工具</Label>
                            <Select value={selectedId} onValueChange={setSelectedId}>
                                <SelectTrigger className="w-full">
                                    <SelectValue placeholder="选择一个内置工具" />
                                </SelectTrigger>
                                <SelectContent>
                                    {templates.map((t) => (
                                        <SelectItem key={t.id} value={t.id}>
                                            {t.name}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                    )}

                    {/* Readonly basics as plain text */}
                    <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                            <div className="text-muted-foreground">ID</div>
                            <div className="text-foreground break-all">
                                {editing ? initialName || "" : selected?.id ?? ""}
                            </div>
                        </div>
                        <div>
                            <div className="text-muted-foreground">类型</div>
                            <div className="text-foreground">{editing ? "stdio" : selected?.transport_type ?? ""}</div>
                        </div>
                        <div className="col-span-2">
                            <div className="text-muted-foreground">描述</div>
                            <div className="text-foreground whitespace-pre-wrap">
                                {editing ? initialDescription || "" : selected?.description ?? ""}
                            </div>
                        </div>
                        <div className="col-span-2">
                            <div className="text-muted-foreground">命令</div>
                            <div className="text-foreground break-all font-mono">
                                {editing ? initialCommand || "" : selected?.command ?? ""}
                            </div>
                        </div>
                    </div>

                    {/* Environment Variables */}
                    <div className="space-y-4">
                        <div className="flex items-center justify-between">
                            <Label className="text-base font-semibold">环境变量配置</Label>
                            {!editing && selected?.required_envs?.some((e) => e.required) && (
                                <div className="text-xs text-muted-foreground">
                                    必填字段:{" "}
                                    {selected.required_envs
                                        .filter((e) => e.required)
                                        .map((e) => e.label)
                                        .join(", ")}
                                </div>
                            )}
                        </div>

                        <Card>
                            <CardContent className="p-6">
                                {templateForFields?.required_envs?.length ? (
                                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                        {templateForFields.required_envs.map(renderEnvField)}
                                    </div>
                                ) : (
                                    <p className="text-muted-foreground text-center py-4">
                                        {templateForFields ? "该工具无需配置环境变量" : "加载环境变量配置中..."}
                                    </p>
                                )}
                            </CardContent>
                        </Card>
                    </div>
                </div>
                <DialogFooter className="flex-shrink-0">
                    <Button variant="ghost" onClick={onClose}>
                        取消
                    </Button>
                    <Button onClick={handleSubmit} disabled={busy}>
                        {editing ? "保存" : "添加"}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default BuiltinToolDialog;
