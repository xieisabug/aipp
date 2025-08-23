import React from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";
import { Button } from "../ui/button";
import { DialogHeader, DialogFooter, Dialog, DialogTrigger, DialogContent, DialogTitle } from "../ui/dialog";
import { Input } from "../ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "../ui/form";
import { AssistantType } from "../../types/assistant";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { AssistantDetail } from "../../data/Assistant";
import { PlusCircle } from "lucide-react";

interface AddAssistantDialogProps {
    assistantTypes: AssistantType[];
    onAssistantAdded: (assistantDetail: AssistantDetail) => void;
    triggerButtonProps?: React.ComponentProps<typeof Button>;
}

const AddAssistantDialog: React.FC<AddAssistantDialogProps> = ({
    assistantTypes,
    onAssistantAdded,
    triggerButtonProps,
}) => {
    const [openAddAssistantDialog, setOpenAddAssistantDialog] = React.useState<boolean>(false);

    const formSchema = z.object({
        name: z.string().min(1, "名称不能为空"),
        description: z.string(),
        assistantType: z.string(),
    });

    const form = useForm<z.infer<typeof formSchema>>({
        resolver: zodResolver(formSchema),
        defaultValues: {
            name: "初始化助手名称",
            description: "这是一个初始化的描述",
            assistantType: "0",
        },
    });

    const onSubmit = (values: z.infer<typeof formSchema>) => {
        invoke<AssistantDetail>("add_assistant", {
            name: values.name,
            description: values.description,
            assistantType: parseInt(values.assistantType),
        })
            .then((assistantDetail: AssistantDetail) => {
                onAssistantAdded(assistantDetail);
                setOpenAddAssistantDialog(false);
                toast.success("新增助手成功");
            })
            .catch((error) => {
                toast.error("新增助手失败: " + error);
            });
    };

    console.log("add assistant dialog", assistantTypes);

    return (
        <Dialog open={openAddAssistantDialog} onOpenChange={setOpenAddAssistantDialog}>
            <DialogTrigger asChild>
                <Button {...triggerButtonProps}>
                    <PlusCircle className="h-4 w-4" />
                </Button>
            </DialogTrigger>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>新增助手</DialogTitle>
                </DialogHeader>
                <Form {...form}>
                    <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                        <FormField
                            control={form.control}
                            name="name"
                            render={({ field }) => (
                                <FormItem>
                                    <FormLabel>名称</FormLabel>
                                    <FormControl>
                                        <Input {...field} />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />
                        <FormField
                            control={form.control}
                            name="description"
                            render={({ field }) => (
                                <FormItem>
                                    <FormLabel>描述</FormLabel>
                                    <FormControl>
                                        <Input {...field} />
                                    </FormControl>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />
                        <FormField
                            control={form.control}
                            name="assistantType"
                            render={({ field }) => (
                                <FormItem>
                                    <FormLabel>类型</FormLabel>
                                    <Select onValueChange={field.onChange} defaultValue={field.value}>
                                        <FormControl>
                                            <SelectTrigger>
                                                <SelectValue placeholder="选择助手类型" />
                                            </SelectTrigger>
                                        </FormControl>
                                        <SelectContent>
                                            <SelectGroup>
                                                {assistantTypes.map((type) => (
                                                    <SelectItem key={type.code} value={type.code.toString()}>
                                                        {type.name}
                                                    </SelectItem>
                                                ))}
                                            </SelectGroup>
                                        </SelectContent>
                                    </Select>
                                    <FormMessage />
                                </FormItem>
                            )}
                        />
                        <DialogFooter>
                            <Button type="submit">确认</Button>
                        </DialogFooter>
                    </form>
                </Form>
            </DialogContent>
        </Dialog>
    );
};

export default AddAssistantDialog;
