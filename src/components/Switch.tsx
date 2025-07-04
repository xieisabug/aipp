import React from 'react';

interface SwitchProps {
    state?: boolean;
    onChange?: () => void;
}

const Switch: React.FC<SwitchProps> = ({ state = false, onChange }) => {

    const handleToggle = () => {
        if (onChange) {
            onChange();
        }
    };

    return (
        <div 
            className="relative inline-block w-[60px] h-[22px] cursor-pointer"
            onClick={handleToggle}
        >
            <div className={`absolute top-0 left-0 right-0 bottom-0 bg-white rounded-[22px] transition-all duration-300 ${state ? 'bg-white' : 'bg-white'}`}>
                <div 
                    className={`absolute content-[''] h-3.5 w-3.5 left-1.5 bottom-1 rounded-full transition-all duration-300 ${
                        state 
                            ? 'transform translate-x-8 bg-primary' 
                            : 'transform translate-x-0 bg-gray-400'
                    }`}
                />
            </div>
        </div>
    );
};

export default Switch;