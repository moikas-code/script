"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.debounce = void 0;
function debounce(callback, delay) {
    let timeout;
    return ((...args) => {
        if (timeout !== undefined) {
            clearTimeout(timeout);
        }
        timeout = setTimeout(() => {
            timeout = undefined;
            callback(...args);
        }, delay);
    });
}
exports.debounce = debounce;
//# sourceMappingURL=timing.js.map