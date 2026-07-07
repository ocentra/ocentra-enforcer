import React from "react";

interface WidgetProps {
    name: string;
}

class Widget extends React.Component<WidgetProps> {
    render() {
        if (this.props.name === "") {
            return <span>unnamed</span>;
        }
        return <span>{helper(this.props.name)}</span>;
    }
}

function helper(label: string): string {
    return label.toUpperCase();
}
