
// D&D process
(() => {
    const input = document.querySelector('.page-input-text-area');

    for (const event of ['dragenter', 'dragover', 'dragleave', 'drop'])
        input.addEventListener(event, e => e.preventDefault(), false);

    input.addEventListener('drop', e => {
        const file = e.dataTransfer.files[0];
        const reader = new FileReader();

        reader.onload = () => {
            input.value = reader.result;
            document.querySelector('.page-input-parse-button').click();
        };

        reader.readAsText(file);
    });
})();
// button "Parse Log" and "Back"
(() => {
    const pageInput = document.querySelector(".page-input");
    const pageLogs = document.querySelector(".page-logs");

    /** @type {HTMLTemplateElement} */
    const template = document.querySelector("#page-log-template-element");

    const logList = document.querySelector(".page-logs-list");

    const input = document.querySelector('.page-input-text-area');
    document.querySelector(".page-input-parse-button").addEventListener('click', (e) => {
        e.stopPropagation();

        const children = [];
        let parsed;
        try {
            parsed = parseContent(input.value);
        } catch (e) {
            alert(`Error parsing log: ${e?.message}`)
            return;
        }
        window.currentLog = parsed;

        const elementByText = {};

        for (const section of parsed.sections) {
            if (section.contentType !== "log-element") continue;

            // sections
            // TODO: log type detection
            const cloned = template.content.cloneNode(true).querySelector(".page-logs-element");
            cloned.dataset.logLevel = getLogLevel(parseInt(section.getField('Mode-Raw'), 16));
            const lines = section.content.split(/\r?\n/g);
            cloned.querySelector(".page-logs-element-text-short").textContent = lines.length === 1 ? lines[0] : `${lines[0]}\n${lines[1]}`;
            cloned.dataset.fullText = section.content;
            // collapse
            let originForText = elementByText[section.content]
            if (originForText != null) {
                cloned.dataset.collapsed = 'true';
                const collasedCount = originForText.querySelector(".page-logs-element-collapsed-count");
                collasedCount.textContent = parseInt(collasedCount.textContent) + 1;
            } else {
                originForText = elementByText[section.content] = cloned;
                const collasedCount = originForText.querySelector(".page-logs-element-collapsed-count");
                collasedCount.textContent = 1;
            }
            children.push(cloned);
        }
        logList.replaceChildren(...children);
        document.querySelector(".page-logs-body").textContent = '';

        pageInput.hidden = true;
        pageLogs.hidden = false;
    })

    document.querySelector(".page-logs-back-button").addEventListener('click', (e) => {
        e.stopPropagation();

        window.currentLog = null;
        pageInput.hidden = false;
        pageLogs.hidden = true;
    })

    const Mode = {
        Error: 1 << 0,
        Assert: 1 << 1,
        Log: 1 << 2,
        Fatal: 1 << 4,
        DontPreprocessCondition: 1 << 5,
        AssetImportError: 1 << 6,
        AssetImportWarning: 1 << 7,
        ScriptingError: 1 << 8,
        ScriptingWarning: 1 << 9,
        ScriptingLog: 1 << 10,
        ScriptCompileError: 1 << 11,
        ScriptCompileWarning: 1 << 12,
        StickyError: 1 << 13,
        MayIgnoreLineNumber: 1 << 14,
        ReportBug: 1 << 15,
        DisplayPreviousErrorInStatusBar: 1 << 16,
        ScriptingException: 1 << 17,
        DontExtractStacktrace: 1 << 18,
        ShouldClearOnPlay: 1 << 19,
        GraphCompileError: 1 << 20,
        ScriptingAssertion: 1 << 21,
        VisualScriptingError: 1 << 22
    }
    function getLogIcon(mode) {
        if ((mode & (Mode.Fatal | Mode.Assert | Mode.Error | Mode.ScriptingError | Mode.AssetImportError | Mode.ScriptCompileError | Mode.GraphCompileError | Mode.ScriptingAssertion | Mode.ScriptingException)) !== 0) {
            return 'error.svg';
        }
        if ((mode & (Mode.ScriptCompileWarning | Mode.ScriptingWarning | Mode.AssetImportWarning)) !== 0) {
            return 'warning.svg';
        }
        return 'info.svg'
    }
    function getLogLevel(mode) {
        if ((mode & (Mode.Fatal | Mode.Assert | Mode.Error | Mode.ScriptingError | Mode.AssetImportError | Mode.ScriptCompileError | Mode.GraphCompileError | Mode.ScriptingAssertion | Mode.ScriptingException)) !== 0) {
            return 'error';
        }
        if ((mode & (Mode.ScriptCompileWarning | Mode.ScriptingWarning | Mode.AssetImportWarning)) !== 0) {
            return 'warning';
        }
        return 'info'
    }
})();

(() => {
    const logBody = document.querySelector(".page-logs-body");
    const logList = document.querySelector(".page-logs-list");
    window.onClickLogElement = (element) => {
        logBody.textContent = element.dataset.fullText;
    };

    document.querySelector(".page-logs-show-project-info-button").addEventListener('click', (e) => {
        e.stopPropagation();

        logBody.textContent = createProjectInfo(window.currentLog.headerValues);
    })

    const collapseButton = document.querySelector(".page-logs-collapse-button");
    collapseButton.addEventListener('click', (e) => {
        if (collapseButton.dataset.active != null) {
            delete collapseButton.dataset.active;
            delete logList.dataset.collapsed;
        } else {
            logList.dataset.collapsed = 'true'
            collapseButton.dataset.active = 'true';
        }
    })

    for (const kind of ['info', 'warning', 'error']) {
        const button = document.querySelector(`.page-logs-${kind}-button`);
        button.addEventListener('click', (e) => {
            if (button.dataset.active != null) {
                delete button.dataset.active;
                logList.setAttribute(`data-hide-${kind}`, 'true');
            } else {
                button.dataset.active = 'true';
                delete logList.removeAttribute(`data-hide-${kind}`, 'true');
            }
        });
    }    

    /// @param headers {[string, string][]}
    function createProjectInfo(headers) {
        const packages = {};
        let unityVersion = null;
        let currentBuildTarget = null;
        let editorPlatform

        for (const [name, body] of headers) {
            switch (name.toLowerCase()) {
                case 'upm-dependency': {
                    const [name, source] = body.split('@');
                    packages[name] ??= {};
                    packages[name].upm = source;
                    break;
                }
                case 'vpm-dependency': {
                    const [name, version] = body.split('@');
                    packages[name] ??= {};
                    packages[name].vpm = version;
                    break;
                }

                case 'unity-version': {
                    unityVersion = body;
                    break;
                }
                case 'build-target': {
                    currentBuildTarget = body;
                    break;
                }
                case 'editor-platform': {
                    editorPlatform = body;
                    break;
                }
            }
        }

        let result = '';
        result += `Unity version: ${unityVersion || "unknown"}\n`;
        result += `Build target: ${currentBuildTarget || "unknown"}\n`;
        result += `Editor platform: ${editorPlatform || "unknown"}\n`;
        result += '\n';
        for (const [name, value] of Object.entries(packages)) {
            result += `${name}:\n`;
            result += `UPM: ${value.upm || "not installed"}\n`;
            if (value.vpm) result += `VPM: ${value.vpm}\n`;
            result += '\n';
        }
        return result;
    }
})();

/**
 *
 * @param content {string}
 * @return {{headerValues: [string, string][], sections: Section[]}}
 */
function parseContent(content) {
    let lines = content.split(/\r\n|\n/);
    if (!lines[0].startsWith('ConsoleLogSaverData/1.')) throw new Error(`Unsupported version`);
    lines = lines.slice(1);
    const headerFieldsEnd = lines.indexOf("");

    /**
     * @param s {string}
     * @param line {number} line index
     */
    const parseHeaderValue = (s, line) => {
        const colon = s.indexOf(':');
        if (colon === -1) throw new Error(`invalid field at line ${line + 1}`);
        const name = s.substring(0, colon);
        let value = s.substring(colon + 1);
        if (value.startsWith(' '))
            value = value.substring(1);
        return [name, value];
    };

    const headerValues = lines.slice(0, headerFieldsEnd).map(parseHeaderValue);

    const separator = headerValues.find(([name, _]) => name.toLowerCase() === 'separator')?.[1];
    if (separator == null) throw new Error(`Separator not found`);

    let lineIndex = lines.findIndex((x, i) => i > headerFieldsEnd && x.endsWith(separator));
    while (true) {
        if (lines[lineIndex].endsWith(separator)) break;
        lineIndex++;
        if (lineIndex === lines.length) // section not found
            return { headerValues, sections: [] };
    }
    lineIndex++;

    /** @type { [int, string][] } */
    let sectionSources = [];

    while (lineIndex < lines.length) {
        const sectionStart = lineIndex;
        while (lineIndex < lines.length && !lines[lineIndex].endsWith(separator)) lineIndex++;
        if (lineIndex < lines.length) lineIndex++;
        // now at section end
        let sectionLines = lines.slice(sectionStart, lineIndex);
        if (sectionLines.every(x => x === '')) break;
        let section = sectionLines.join('\n');
        section = section.slice(0, section.length - separator.length);
        sectionSources.push([sectionStart, section]);
    }

    // parse each section

    /**
     * @param firstLine {number}
     * @param source {string}
     * @return {Section}
     */
    const parseSection = ([firstLine, source]) => {
        const lines = source.split('\n');
        const fieldsEnd = lines.indexOf('');

        const fieldValues = lines.slice(0, fieldsEnd).map((line, i) => parseHeaderValue(line, i + firstLine));
        const content = lines.slice(fieldsEnd + 1).join('\n');

        return new Section(fieldValues, content);
    };

    const sections = sectionSources.map(parseSection);

    return {headerValues, sections};
}

class Section {
    /** @type {[string, string][]} */
    fields;
    /** @type {string} */
    content;
    /**
     * @param fields {[string, string][]}
     * @param content {string}
     */
    constructor(fields, content) {
        this.fields = fields;
        this.content = content;
    }

    /** @type {string | null} */
    get contentType() {
        return this.getField("content")
    }

    /**
     * @param name {string}
     * @return {string | null}
     */
    getField(name) {
        name = name.toLowerCase();
        return this.fields.find(x => x[0].toLowerCase() === name)?.[1]
    }
}
