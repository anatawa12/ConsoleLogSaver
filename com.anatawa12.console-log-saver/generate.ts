console.log("// generated")
console.log("// ReSharper disable InconsistentNaming")
console.log("// ReSharper disable RedundantNameQualifier")
console.log("namespace anatawa12.gists")
console.log("{")
console.log("    static partial class LogEntries")
console.log("    {")
genSFunc("void", "RowGotDoubleClicked", [["int", "index"]]);
genSFunc("void", "OpenFileOnSpecificLineAndColumn", [["string", "filePath"], ["int", "line"], ["int", "column"]]);
genSFunc("string", "GetStatusText", []);
genSFunc("int", "GetStatusMask", []);
genSFunc("int", "StartGettingEntries", []);
genSProp("int", "consoleFlags");
genSFunc("void", "SetConsoleFlag", [["int", "bit"], ["bool", "value"]]);
genSFunc("void", "SetFilteringText", [["string", "filteringText"]]);
genSFunc("string", "GetFilteringText", []);
genSFunc("void", "EndGettingEntries", []);
genSFunc("int", "GetCount", []);
genSFunc("void", "GetCountsByType", [["ref", "int", "errorCount"], ["ref", "int", "warningCount"], ["ref", "int", "logCount"]]);
genSFunc("void", "GetLinesAndModeFromEntryInternal", [["int", "row"], ["int", "numberOfLines"], ["ref", "int", "mask"], ["ref", "string", "outString"]]);
genSFunc("bool", "GetEntryInternal", [["int", "row"], ["LogEntry", "outputEntry"]]);
genSFunc("int", "GetEntryCount", [["int", "row"]]);
genSFunc("void", "Clear", []);
genSFunc("int", "GetStatusViewErrorIndex", []);
genSFunc("void", "ClickStatusBar", [["int", "count"]]);
genSFunc("void", "AddMessageWithDoubleClickCallback", [["LogEntry", "outputEntry"]]);
console.log("    }")
console.log("    partial struct LogEntry")
console.log("    {")
genIField("string", "message");
genIField("string", "file");
genIField("int", "line");
genIField("int", "column");
genIField("int", "mode");
genIField("int", "instanceID");
genIField("int", "identifier");
console.log("    }")
console.log("}")

function genSFunc(returns: string, name: string, params: ([string, string] | ["ref", string, string])[]) {
    console.log(`        public static ${returns} ${name}(`)
    for (let i = 0; i < params.length; i++) {
        const param = params[i];
        const comma = i == params.length - 1 ? "" : ", ";
        if (param[0] == "ref") {
            console.log(`            ref ${param[1]} ${param[2]}${comma}`)
        } else {
            console.log(`            ${param[0]} ${param[1]}${comma}`)
        }
    }
    console.log(`        ) => ${name}Method(`)
    for (let i = 0; i < params.length; i++) {
        const param = params[i];
        const comma = i == params.length - 1 ? "" : ", ";
        if (param[0] == "ref") {
            console.log(`            ref ${param[2]}${comma}`)
        } else {
            console.log(`            ${param[1]}${comma}`)
        }
    }
    console.log(`        );`)
    console.log(`        private static readonly ${name}Delegate ${name}Method = ReflectionWrapperUtil.CreateStaticMethod<${name}Delegate>(`)
    console.log(`            BackedType,`)
    console.log(`            "${name}",`)
    console.log(`            new global::System.Type[] {`)
    for (let param of params) {
        if (param[0] == "ref") {
            console.log(`                typeof(${param[1]}).MakeByRefType(), `)
        } else {
            console.log(`                typeof(${param[0]}), `)
        }
    }
    console.log(`            });`)
    console.log(`        delegate ${returns} ${name}Delegate(`)
    for (let i = 0; i < params.length; i++) {
        const param = params[i];
        const comma = i == params.length - 1 ? "" : ", ";
        if (param[0] == "ref") {
            console.log(`                ref ${param[1]} ${param[2]}${comma}`)
        } else {
            console.log(`                ${param[0]} ${param[1]}${comma}`)
        }
    }
    console.log(`        );`)
}

function genIField(type: string, name: string) {
    console.log(`        public ${type} ${name}`)
    console.log(`        {`)
    console.log(`            get => ${name}Getter(BackedValue);`);
    console.log(`            set => ${name}Setter(BackedValue, value);`);
    console.log(`        }`)
    console.log(`        private static readonly ${name}GetterDelegate ${name}Getter =`)
    console.log(`            ReflectionWrapperUtil.CreateInstanceFieldGetter<${name}GetterDelegate>(BackedType, typeof(${type}), "${name}");`)
    console.log(`        private static readonly ${name}SetterDelegate ${name}Setter =`)
    console.log(`            ReflectionWrapperUtil.CreateInstanceFieldSetter<${name}SetterDelegate>(BackedType, typeof(${type}), "${name}");`)
    console.log(`        delegate ${type} ${name}GetterDelegate(object self);`)
    console.log(`        delegate void ${name}SetterDelegate(object self, ${type} value);`)
}

function genSProp(type: string, name: string) {
    console.log(`        public static ${type} ${name}`)
    console.log(`        {`)
    console.log(`            get => ${name}Getter();`);
    console.log(`            set => ${name}Setter(value);`);
    console.log(`        }`)
    console.log(`        private static readonly ${name}GetterDelegate ${name}Getter =`)
    console.log(`            ReflectionWrapperUtil.CreateStaticPropertyGetter<${name}GetterDelegate>(BackedType, typeof(${type}), "${name}");`)
    console.log(`        private static readonly ${name}SetterDelegate ${name}Setter =`)
    console.log(`            ReflectionWrapperUtil.CreateStaticPropertySetter<${name}SetterDelegate>(BackedType, typeof(${type}), "${name}");`)
    console.log(`        delegate ${type} ${name}GetterDelegate();`)
    console.log(`        delegate void ${name}SetterDelegate(${type} value);`)
}
