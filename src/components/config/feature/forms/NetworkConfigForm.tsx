import React, { useCallback } from "react";
import { UseFormReturn } from "react-hook-form";
import ConfigForm from "@/components/ConfigForm";
import { toast } from "sonner";

interface NetworkConfigFormProps {
    form: UseFormReturn<any>;
    onSave: () => Promise<void>;
}

export const NetworkConfigForm: React.FC<NetworkConfigFormProps> = ({ form, onSave }) => {
    const handleSaveNetwork = useCallback(async () => {
        try {
            await onSave();
            toast.success("网络配置保存成功");
        } catch (e) {
            toast.error("保存网络配置失败: " + e);
        }
    }, [onSave]);

    const NETWORK_FORM_CONFIG = [
        {
            key: "request_timeout",
            config: {
                type: "input" as const,
                label: "请求超时时间（秒）",
                placeholder: "180",
                description: "思考模型返回较慢，不建议设置过低",
            },
        },
        {
            key: "retry_attempts",
            config: {
                type: "input" as const,
                label: "失败重试次数",
                placeholder: "3",
                description: "请求失败时的重试次数",
            },
        },
        {
            key: "network_proxy",
            config: {
                type: "input" as const,
                label: "网络代理",
                placeholder: "http://127.0.0.1:7890",
                description: "支持 http、https 和 socks 协议，例如：http://127.0.0.1:7890",
            },
        },
    ];

    return (
        <ConfigForm
            title="网络配置"
            description="配置请求超时、重试次数和网络代理"
            config={NETWORK_FORM_CONFIG}
            layout="default"
            classNames="bottom-space"
            useFormReturn={form}
            onSave={handleSaveNetwork}
        />
    );
};