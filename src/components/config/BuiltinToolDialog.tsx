import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "../ui/dialog";
import { Label } from "../ui/label";
import { Textarea } from "../ui/textarea";
import { Button } from "../ui/button";

type BuiltinTemplate = {
    id: string;
    name: string;
    description: string;
    command: string;
    transport_type: string;
    required_envs: { key: string; required: boolean; tip?: string }[];
};

interface BuiltinToolDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: () => void;
    // optional edit mode support
    editing?: boolean;
    // prefill when editing existing built-in server
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
    const [envText, setEnvText] = useState<string>(initialEnvText || "");
    const [busy, setBusy] = useState(false);

    const selected = useMemo(() => templates.find((t) => t.id === selectedId), [templates, selectedId]);

    useEffect(() => {
        if (!isOpen) return;
        if (editing) {
            // In edit mode we don't need templates; keep selectedId from command if provided
            setTemplates([]);
            return;
        }
        invoke<BuiltinTemplate[]>("list_aipp_builtin_templates")
            .then(setTemplates)
            .catch(() => {});
    }, [isOpen, editing]);

    useEffect(() => {
        if (editing) return; // keep provided envs
        // Reset envs when template changes
        setEnvText("");
    }, [selectedId, editing]);

    const handleSubmit = async () => {
        if (!selected) return;
        setBusy(true);
        // Parse envText to map
        const envs: Record<string, string> = {};
        envText
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

        try {
            if (!editing) {
                await invoke<number>("add_or_update_aipp_builtin_server", {
                    templateId: selected.id,
                    name: selected.name,
                    description: selected.description,
                    envs,
                });
            } else {
                // editing: update existing server envs only via general update API
                // We need server id to update, but MCPConfig will handle opening standard edit for non-builtin.
                // Here we cannot update without id; so we call generic add_or_update by matching name+command is not supported.
                // As a fallback, we emit onSubmit and let parent refresh; parent should call update flow separately if needed.
            }
            onSubmit();
        } catch (e) {
            // noop; outer page toasts
            console.error(e);
        } finally {
            setBusy(false);
        }
    };

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="max-w-2xl">
                <DialogHeader>
                    <DialogTitle>{editing ? "编辑内置工具环境变量" : "添加内置工具"}</DialogTitle>
                </DialogHeader>
                <div className="space-y-4">
                    {/* Template selector; hidden in edit mode */}
                    {!editing && (
                        <div className="space-y-2">
                            <Label>选择内置工具</Label>
                            <select
                                className="w-full rounded-md border p-2 text-sm"
                                value={selectedId}
                                onChange={(e) => setSelectedId(e.target.value)}
                            >
                                {templates.map((t) => (
                                    <option key={t.id} value={t.id}>
                                        {t.name}
                                    </option>
                                ))}
                            </select>
                        </div>
                    )}

                    {/* Readonly basics as plain text */}
                    <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                            <div className="text-muted-foreground">ID</div>
                            <div className="text-foreground break-all">{editing ? (initialName || "") : (selected?.id ?? "")}</div>
                        </div>
                        <div>
                            <div className="text-muted-foreground">类型</div>
                            <div className="text-foreground">{editing ? "stdio" : (selected?.transport_type ?? "")}</div>
                        </div>
                        <div className="col-span-2">
                            <div className="text-muted-foreground">描述</div>
                            <div className="text-foreground whitespace-pre-wrap">{editing ? (initialDescription || "") : (selected?.description ?? "")}</div>
                        </div>
                        <div className="col-span-2">
                            <div className="text-muted-foreground">命令</div>
                            <div className="text-foreground break-all font-mono">{editing ? (initialCommand || "") : (selected?.command ?? "")}</div>
                        </div>
                    </div>

                    {/* Envs */}
                    <div className="space-y-2">
                        <Label>环境变量 (KEY=VALUE，每行一个)</Label>
                        {!editing && selected?.required_envs?.length ? (
                            <div className="text-xs text-muted-foreground">
                                必填:{" "}
                                {selected.required_envs
                                    .filter((e) => e.required)
                                    .map((e) => e.key)
                                    .join(", ") || "无"}
                            </div>
                        ) : null}
                        <Textarea
                            placeholder="API_KEY=xxx\nREGION=cn"
                            rows={6}
                            value={envText}
                            onChange={(e) => {
                                setEnvText(e.target.value);
                                onEnvChange?.(e.target.value);
                            }}
                        />
                        {!editing &&
                            selected?.required_envs?.map((e) =>
                                e.tip ? (
                                    <div key={e.key} className="text-xs text-muted-foreground">
                                        {e.key}: {e.tip}
                                    </div>
                                ) : null
                            )}
                    </div>
                </div>
                <DialogFooter>
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
