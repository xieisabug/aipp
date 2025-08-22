import React, { useCallback, useRef } from "react";
import { UseFormReturn } from "react-hook-form";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { emit } from "@tauri-apps/api/event";
import ConfigForm from "@/components/ConfigForm";
import { toast } from "sonner";
import { AVAILABLE_CODE_THEMES } from "@/hooks/useCodeTheme";

interface DisplayConfigFormProps {
    form: UseFormReturn<any>;
    onSave: () => Promise<void>;
}

export const DisplayConfigForm: React.FC<DisplayConfigFormProps> = ({ form, onSave }) => {
    const previousNotificationValue = useRef<boolean | undefined>(undefined);
    
    const themeOptions = [{ value: "default", label: "默认主题" }];

    const colorModeOptions = [
        { value: "light", label: "浅色" },
        { value: "dark", label: "深色" },
        { value: "system", label: "跟随系统" },
    ];

    const markdownRenderOptions = [
        { value: "enabled", label: "开启" },
        { value: "disabled", label: "关闭" },
    ];

    // 代码主题选项
    const lightCodeThemeOptions = AVAILABLE_CODE_THEMES.filter((theme) => theme.category === "light").map((theme) => ({
        value: theme.id,
        label: theme.name,
    }));

    const darkCodeThemeOptions = AVAILABLE_CODE_THEMES.filter((theme) => theme.category === "dark").map((theme) => ({
        value: theme.id,
        label: theme.name,
    }));

    const handleSaveDisplayConfig = useCallback(async () => {
        const values = form.getValues();
        const currentNotificationValue = values.notification_on_completion;

        // 检查通知设置是否从 false 变为 true
        const notificationJustEnabled = 
            previousNotificationValue.current === false && 
            currentNotificationValue === true;

        // 如果用户刚刚开启了通知，需要检查和申请权限
        if (notificationJustEnabled) {
            try {
                let permissionGranted = await isPermissionGranted();

                if (!permissionGranted) {
                    const permission = await requestPermission();
                    permissionGranted = permission === "granted";
                }

                if (!permissionGranted) {
                    toast.error("通知权限未获取，无法开启系统通知功能");
                    // 重置开关状态
                    form.setValue("notification_on_completion", false);
                    return;
                }

                // 权限获取成功，发送测试通知
                sendNotification({
                    title: "AIPP - 系统通知已开启",
                    body: "AI 消息完成时将发送系统通知",
                });
                toast.success("通知权限获取成功，已发送测试通知");
            } catch (e) {
                toast.error("获取通知权限时发生错误: " + e);
                form.setValue("notification_on_completion", false);
                return;
            }
        }

        try {
            await onSave();

            // 更新上次的通知设置值
            previousNotificationValue.current = currentNotificationValue;

            // 发出主题变化事件，通知其他窗口和组件
            await emit("theme-changed", {
                mode: values.color_mode,
                code_theme_light: values.code_theme_light,
                code_theme_dark: values.code_theme_dark,
            });

            toast.success("显示配置保存成功");
        } catch (e) {
            toast.error("保存显示配置失败: " + e);
        }
    }, [form, onSave]);

    const DISPLAY_FORM_CONFIG = [
        {
            key: "theme",
            config: {
                type: "select" as const,
                label: "系统外观主题",
                options: themeOptions,
            },
        },
        {
            key: "color_mode",
            config: {
                type: "select" as const,
                label: "深浅色模式",
                options: colorModeOptions,
            },
        },
        {
            key: "code_theme_light",
            config: {
                type: "select" as const,
                label: "浅色模式代码主题",
                options: lightCodeThemeOptions,
            },
        },
        {
            key: "code_theme_dark",
            config: {
                type: "select" as const,
                label: "深色模式代码主题",
                options: darkCodeThemeOptions,
            },
        },
        {
            key: "user_message_markdown_render",
            config: {
                type: "select" as const,
                label: "用户消息Markdown渲染",
                options: markdownRenderOptions,
            },
        },
        {
            key: "notification_on_completion",
            config: {
                type: "switch" as const,
                label: "消息完成时发送系统通知",
                tooltip: "AI消息生成完成时发送系统通知提醒",
            },
        },
    ];

    return (
        <ConfigForm
            title="显示"
            description="配置系统外观主题、深浅色模式和用户消息渲染方式"
            config={DISPLAY_FORM_CONFIG}
            layout="default"
            classNames="bottom-space"
            useFormReturn={form}
            onSave={handleSaveDisplayConfig}
        />
    );
};
