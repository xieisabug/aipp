import React, { useState, useEffect, useRef, useMemo } from "react";
import { Controller, SubmitHandler, UseFormReturn } from "react-hook-form";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import IconButton from "./IconButton";
import Copy from "../assets/copy.svg?react";
import Delete from "../assets/delete.svg?react";
import Edit from "../assets/edit.svg?react";
import "../styles/ConfigForm.css";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { Form, FormControl, FormItem, FormLabel, FormMessage } from "./ui/form";
import { Button } from "./ui/button";
import { Textarea } from "./ui/textarea";
import { RadioGroup, RadioGroupItem } from "./ui/radio-group";
import { Input } from "./ui/input";
import { Checkbox } from "./ui/checkbox";

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
	| "button"; // 添加 "button" 类型
	label: string;
	className?: string;
	options?: { value: string; label: string; tooltip?: string }[];
	value?: string | boolean;
	tooltip?: string;
	onChange?: (value: string | boolean) => void;
	onBlur?: (value: string | boolean) => void;
	customRender?: (fieldRenderData: any) => React.ReactNode;
	onClick?: (assistantConfigApi?: AssistantConfigApi) => void; // 为按钮添加 onClick 处理函数
}

interface ConfigFormProps {
	title: string;
	description?: string;
	config: Record<string, ConfigField>;
	classNames?: string;
	// 是否可展开
	enableExpand?: boolean;
	// 是否默认展开
	defaultExpanded?: boolean;
	useFormReturn: UseFormReturn<any, any, undefined>;
	assistantConfigApi?: AssistantConfigApi;
	/**
	 * default 直接从上向下展示所有的配置项
	 * prompt 会单独将prompt配置项放在右侧
	 * provider 会单独将modelList配置项放在右侧
	 */
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
	// console.log("ConfigForm render");

	// // 在组件顶部添加 props 变化追踪
	// useEffect(() => {
	// 	console.log("ConfigForm props changed", {
	// 		title,
	// 		description,
	// 		config,
	// 		classNames,
	// 		enableExpand,
	// 		defaultExpanded,
	// 		layout,
	// 		useFormReturn,
	// 		onSave,
	// 		onCopy,
	// 		onDelete,
	// 		onEdit,
	// 		extraButtons,
	// 	});
	// }, [config, useFormReturn, onSave, onCopy, onDelete, onEdit, extraButtons]);

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

	const CustomFormField = React.memo(({ field, name }: { field: ConfigField, name: string }) => {
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
									<SelectItem key={option.value} value={option.value}>
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
							type={field.type === "password" ? "password" : "text"}
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
							defaultValue={fieldRenderData.value}
							onValueChange={fieldRenderData.onChange}
						>
							{field.options?.map((option) => (
								<FormItem className="flex items-center space-x-2">
									<FormControl>
										<RadioGroupItem value={option.value} id={option.value} />
									</FormControl>
									<FormLabel className="font-normal" htmlFor={option.value}>{option.label}</FormLabel>
									{option.tooltip && (
										<span className="tooltip-trigger" title={field.tooltip}>
											?
										</span>
									)}
								</FormItem>
							))}
						</RadioGroup>
					);
				case "static":
					return <div className={field.className}>{field.value}</div>;
				case "custom":
					console.log("create custom")
					const customElement = useMemo(() => {
						return field.customRender ? field.customRender(fieldRenderData) : null;
					}, [field.customRender, fieldRenderData]);
					return customElement;
				case "button":
					return (
						<Button type="button" className={field.className} onClick={() => {
							console.log("ConfigForm button clicked", field);
							field.onClick && field.onClick(assistantConfigApi);
						}}>{field.value as string}</Button>
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
					<FormItem className="form-group">
						<FormLabel>{field.label}</FormLabel>
						<FormControl>
							{renderField(fieldRenderData)}
						</FormControl>
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
					<div className="assistant-config-grid">
						<div className="assistant-config-properties">
							{Object.entries(config)
								.filter((k) => k[0] !== "prompt")
								.map(([key, field]) => (
									<CustomFormField name={key} field={field} key={key} />
								))}
						</div>
						{config.prompt && (
							<div className="assistant-config-prompts">
								<CustomFormField name="prompt" field={config.prompt} />
							</div>
						)}
					</div>
				);
			case "provider":
				return (
					<div className="provider-config-item-form">
						<div className="provider-config-item-form-property-container">
							{Object.entries(config).map(([key, field]) => (
								<CustomFormField name={key} field={field} key={key} />
							))}
						</div>
						{config.modelList && (
							<div className="provider-config-item-form-model-list-container">
								<CustomFormField name="model_list" field={config.modelList} />
							</div>
						)}
					</div>
				);
			default:
				return (
					<div>
						{Object.entries(config).map(([key, field]) => (
							<CustomFormField name={key} field={field} key={key} />
						))}
					</div>
				);
		}
	};

	return (
		<Card className={classNames ? classNames + " config-window-container" : "config-window-container"}>
			<CardHeader onClick={toggleExpand} className="flex flex-row items-center cursor-pointer">
				<div className="grid gap-2">
					<CardTitle>{title}</CardTitle>
					<CardDescription>{description}</CardDescription>
				</div>
				<div className="flex items-center ml-auto gap-1">
					{onCopy && <IconButton icon={<Copy fill="black" />} onClick={onCopy} />}
					{onDelete && <IconButton icon={<Delete fill="black" />} onClick={onDelete} />}
					{onEdit && <IconButton icon={<Edit fill="black" />} onClick={onEdit} />}
					{extraButtons}
				</div>
			</CardHeader>

			<CardContent ref={contentRef} className={`config-window-content ${isExpanded ? "expanded" : ""}`}>
				<Form {...useFormReturn}>
					{renderContent()}
					{onSave && (
						<div>
							<Button onClick={onSave}>保存</Button>
						</div>
					)}
				</Form>
			</CardContent>
		</Card >
	);
};

export default React.memo(ConfigForm);
