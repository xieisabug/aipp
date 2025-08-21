import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

export const useVersionManager = () => {
    // Bun 相关状态
    const [bunVersion, setBunVersion] = useState<string>("");
    const [isInstallingBun, setIsInstallingBun] = useState(false);
    const [bunInstallLog, setBunInstallLog] = useState("");

    // UV 相关状态
    const [uvVersion, setUvVersion] = useState<string>("");
    const [isInstallingUv, setIsInstallingUv] = useState(false);
    const [uvInstallLog, setUvInstallLog] = useState("");

    // 检查 Bun 版本
    const checkBunVersion = useCallback(() => {
        invoke("check_bun_version").then((version) => {
            setBunVersion(version as string);
        });
    }, []);

    // 检查 UV 版本
    const checkUvVersion = useCallback(() => {
        invoke("check_uv_version").then((version) => {
            setUvVersion(version as string);
        });
    }, []);

    // 安装 Bun
    const installBun = useCallback(() => {
        setIsInstallingBun(true);
        setBunInstallLog("开始进行 Bun 安装...");
        invoke("install_bun");
    }, []);

    // 安装 UV
    const installUv = useCallback(() => {
        setIsInstallingUv(true);
        setUvInstallLog("Starting uv installation...");
        invoke("install_uv");
    }, []);

    // 设置事件监听器
    useEffect(() => {
        // 初始检查版本
        checkBunVersion();
        checkUvVersion();

        // 监听 Bun 安装日志
        const unlistenBunLog = listen("bun-install-log", (event) => {
            setBunInstallLog((prev) => prev + "\\n" + event.payload);
        });

        // 监听 Bun 安装完成
        const unlistenBunFinished = listen("bun-install-finished", (event) => {
            setTimeout(() => {
                setIsInstallingBun(false);
            }, 1000);
            if (event.payload) {
                toast.success("Bun 安装成功");
                checkBunVersion();
            } else {
                toast.error("Bun 安装失败");
            }
        });

        // 监听 UV 安装日志
        const unlistenUvLog = listen("uv-install-log", (event) => {
            setUvInstallLog((prev) => prev + "\\n" + event.payload);
        });

        // 监听 UV 安装完成
        const unlistenUvFinished = listen("uv-install-finished", (event) => {
            setTimeout(() => {
                setIsInstallingUv(false);
            }, 1000);
            if (event.payload) {
                toast.success("uv 安装成功");
                checkUvVersion();
            } else {
                toast.error("uv 安装失败");
            }
        });

        // 清理函数
        return () => {
            unlistenBunLog.then((f) => f());
            unlistenBunFinished.then((f) => f());
            unlistenUvLog.then((f) => f());
            unlistenUvFinished.then((f) => f());
        };
    }, [checkBunVersion, checkUvVersion]);

    return {
        // Bun 相关
        bunVersion,
        isInstallingBun,
        bunInstallLog,
        checkBunVersion,
        installBun,
        
        // UV 相关
        uvVersion,
        isInstallingUv,
        uvInstallLog,
        checkUvVersion,
        installUv,
    };
};