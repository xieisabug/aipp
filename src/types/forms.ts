// 表单配置通用类型定义
export interface FormConfigField {
    key: string;
    config: FieldConfig;
}

export interface FieldConfig {
    type: FieldType;
    label: string;
    value?: any;
    options?: SelectOption[];
    placeholder?: string;
    description?: string;
    tooltip?: string;
    className?: string;
    disabled?: boolean;
    hidden?: boolean;
    onChange?: (value: string | boolean) => void;
    onBlur?: (value: string | boolean) => void;
    onClick?: () => void;
    customRender?: () => React.ReactNode;
}

export type FieldType = 
    | "input" 
    | "textarea" 
    | "select" 
    | "checkbox" 
    | "radio"
    | "button" 
    | "static" 
    | "custom"
    | "switch"
    | "password"
    | "model-select";

export interface SelectOption {
    value: string;
    label: string;
}

// Assistant 相关类型
export interface AssistantFormConfig extends FormConfigField {}

// Feature 相关类型  
export interface FeatureFormConfig extends FormConfigField {}

// 对话框状态类型
export interface DialogStates {
    confirmDeleteOpen: boolean;
    updateFormOpen: boolean;
    shareOpen: boolean;
    importOpen: boolean;
}

// 助手配置 API
export interface AssistantConfigApi {
    clearFieldValue: (fieldName: string) => void;
    changeFieldValue: (
        fieldName: string,
        value: any,
        valueType: string,
    ) => void;
}