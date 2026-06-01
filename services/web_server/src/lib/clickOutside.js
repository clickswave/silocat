/** Dispatch event on click outside of node */
export function clickOutside(node, onEventFunction) {
    const handleClick = (event) => {
        if (node && !node.contains(event.target) && !event.defaultPrevented) {
            onEventFunction();
        }
    };

    document.addEventListener('click', handleClick, true);

    return {
        destroy() {
            document.removeEventListener('click', handleClick, true);
        }
    };
}
