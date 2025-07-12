"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.preventDefault = exports.stopPropagation = void 0;
function stopPropagation(ev) {
    var _a;
    (_a = ev === null || ev === void 0 ? void 0 : ev.stopPropagation) === null || _a === void 0 ? void 0 : _a.call(ev);
}
exports.stopPropagation = stopPropagation;
function preventDefault(ev) {
    var _a;
    (_a = ev === null || ev === void 0 ? void 0 : ev.preventDefault) === null || _a === void 0 ? void 0 : _a.call(ev);
}
exports.preventDefault = preventDefault;
//# sourceMappingURL=dom.js.map