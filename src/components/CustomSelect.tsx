import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check } from 'lucide-react';

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
  className = ""
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

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };

  }, []);

  const handleSelectClick = () => {
    setIsOpen(!isOpen);
  };

  const handleOptionClick = (optionValue: string) => {
    console.log(optionValue)
    onChange(optionValue);
    setIsOpen(false);
  };

  const selectedOption = options.find(option => option.value === value);

  return (
    <div className={`relative ${className}`} ref={selectRef}>
      <button
        type="button"
        className={`
          w-full flex items-center justify-between px-3 py-2 
          bg-white border border-gray-300 rounded-lg shadow-sm
          text-left text-sm text-gray-900
          hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500
          transition-colors duration-200
          ${isOpen ? 'border-gray-500 ring-2 ring-gray-500' : ''}
        `}
        onClick={handleSelectClick}
        title={selectedOption?.label || placeholder}
      >
        <span className="block truncate">
          {selectedOption?.label || placeholder}
        </span>
        <ChevronDown
          className={`
            h-4 w-4 text-gray-400 transition-transform duration-200
            ${isOpen ? 'rotate-180' : ''}
          `}
        />
      </button>

      {isOpen && (
        <div className="absolute z-50 w-full mt-1 bg-white border border-gray-300 rounded-lg shadow-lg max-h-60 overflow-auto">
          {options.map(option => (
            <div
              key={option.value}
              className={`
                flex items-center justify-between px-3 py-2 cursor-pointer text-sm
                hover:bg-gray-50 hover:text-gray-700
                ${option.value === value ? 'bg-gray-50 text-gray-700' : 'text-gray-900'}
              `}
              onClick={() => handleOptionClick(option.value)}
              title={option.label}
            >
              <span className="block truncate">{option.label}</span>
              {option.value === value && (
                <Check className="h-4 w-4 text-gray-600 flex-shrink-0" />
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default CustomSelect;