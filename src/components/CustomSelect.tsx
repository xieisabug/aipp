import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, Check } from "lucide-react";

interface Option {
    value: string;
    label: string;
}

interface CustomSelectProps {
    options: Option[];
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    className?: string;
}

const CustomSelect: React.FC<CustomSelectProps> = ({
    options,
    value,
    onChange,
    placeholder = "请选择...",
    className = "",
}) => {
    const [isOpen, setIsOpen] = useState<boolean>(false);
    const selectRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        console.log(options);

        const handleClickOutside = (event: MouseEvent) => {
            if (selectRef.current && !selectRef.current.contains(event.target as Node)) {
                setIsOpen(false);
            }
        };

        document.addEventListener("mousedown", handleClickOutside);
        return () => {
            document.removeEventListener("mousedown", handleClickOutside);
        };
    }, []);

    const handleSelectClick = () => {
        setIsOpen(!isOpen);
    };

    const handleOptionClick = (optionValue: string) => {
        console.log(optionValue);
        onChange(optionValue);
        setIsOpen(false);
    };

    const selectedOption = options.find((option) => option.value === value);

    return (
        <div className={`relative ${className}`} ref={selectRef}>
            <button
                type="button"
                className={`
          w-full flex items-center justify-between px-3 py-2 
          bg-background border border-border rounded-lg shadow-sm
          text-left text-sm text-foreground
          hover:border-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-primary
          transition-colors duration-200
          ${isOpen ? "border-primary ring-2 ring-primary" : ""}
        `}
                onClick={handleSelectClick}
                title={selectedOption?.label || placeholder}
            >
                <span className="block truncate">{selectedOption?.label || placeholder}</span>
                <ChevronDown
                    className={`
            h-4 w-4 text-muted-foreground transition-transform duration-200
            ${isOpen ? "rotate-180" : ""}
          `}
                />
            </button>

            {isOpen && (
                <div className="absolute z-50 w-full mt-1 bg-background border border-border rounded-lg shadow-lg max-h-60 overflow-auto">
                    {options.map((option) => (
                        <div
                            key={option.value}
                            className={`
                flex items-center justify-between px-3 py-2 cursor-pointer text-sm
                hover:bg-muted hover:text-foreground
                ${option.value === value ? "bg-muted text-foreground" : "text-foreground"}
              `}
                            onClick={() => handleOptionClick(option.value)}
                            title={option.label}
                        >
                            <span className="block truncate">{option.label}</span>
                            {option.value === value && (
                                <Check className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
};

export default CustomSelect;
