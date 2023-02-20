// generated
// ReSharper disable InconsistentNaming
// ReSharper disable RedundantNameQualifier
namespace anatawa12.gists
{
    static partial class LogEntries
    {
        public static void RowGotDoubleClicked(
            int index
        ) => RowGotDoubleClickedMethod(
            index
        );
        private static readonly RowGotDoubleClickedDelegate RowGotDoubleClickedMethod = ReflectionWrapperUtil.CreateStaticMethod<RowGotDoubleClickedDelegate>(
            BackedType,
            "RowGotDoubleClicked",
            new global::System.Type[] {
                typeof(int), 
            });
        delegate void RowGotDoubleClickedDelegate(
                int index
        );
        public static void OpenFileOnSpecificLineAndColumn(
            string filePath, 
            int line, 
            int column
        ) => OpenFileOnSpecificLineAndColumnMethod(
            filePath, 
            line, 
            column
        );
        private static readonly OpenFileOnSpecificLineAndColumnDelegate OpenFileOnSpecificLineAndColumnMethod = ReflectionWrapperUtil.CreateStaticMethod<OpenFileOnSpecificLineAndColumnDelegate>(
            BackedType,
            "OpenFileOnSpecificLineAndColumn",
            new global::System.Type[] {
                typeof(string), 
                typeof(int), 
                typeof(int), 
            });
        delegate void OpenFileOnSpecificLineAndColumnDelegate(
                string filePath, 
                int line, 
                int column
        );
        public static string GetStatusText(
        ) => GetStatusTextMethod(
        );
        private static readonly GetStatusTextDelegate GetStatusTextMethod = ReflectionWrapperUtil.CreateStaticMethod<GetStatusTextDelegate>(
            BackedType,
            "GetStatusText",
            new global::System.Type[] {
            });
        delegate string GetStatusTextDelegate(
        );
        public static int GetStatusMask(
        ) => GetStatusMaskMethod(
        );
        private static readonly GetStatusMaskDelegate GetStatusMaskMethod = ReflectionWrapperUtil.CreateStaticMethod<GetStatusMaskDelegate>(
            BackedType,
            "GetStatusMask",
            new global::System.Type[] {
            });
        delegate int GetStatusMaskDelegate(
        );
        public static int StartGettingEntries(
        ) => StartGettingEntriesMethod(
        );
        private static readonly StartGettingEntriesDelegate StartGettingEntriesMethod = ReflectionWrapperUtil.CreateStaticMethod<StartGettingEntriesDelegate>(
            BackedType,
            "StartGettingEntries",
            new global::System.Type[] {
            });
        delegate int StartGettingEntriesDelegate(
        );
        public static void SetConsoleFlag(
            int bit, 
            bool value
        ) => SetConsoleFlagMethod(
            bit, 
            value
        );
        private static readonly SetConsoleFlagDelegate SetConsoleFlagMethod = ReflectionWrapperUtil.CreateStaticMethod<SetConsoleFlagDelegate>(
            BackedType,
            "SetConsoleFlag",
            new global::System.Type[] {
                typeof(int), 
                typeof(bool), 
            });
        delegate void SetConsoleFlagDelegate(
                int bit, 
                bool value
        );
        public static void SetFilteringText(
            string filteringText
        ) => SetFilteringTextMethod(
            filteringText
        );
        private static readonly SetFilteringTextDelegate SetFilteringTextMethod = ReflectionWrapperUtil.CreateStaticMethod<SetFilteringTextDelegate>(
            BackedType,
            "SetFilteringText",
            new global::System.Type[] {
                typeof(string), 
            });
        delegate void SetFilteringTextDelegate(
                string filteringText
        );
        public static string GetFilteringText(
        ) => GetFilteringTextMethod(
        );
        private static readonly GetFilteringTextDelegate GetFilteringTextMethod = ReflectionWrapperUtil.CreateStaticMethod<GetFilteringTextDelegate>(
            BackedType,
            "GetFilteringText",
            new global::System.Type[] {
            });
        delegate string GetFilteringTextDelegate(
        );
        public static void EndGettingEntries(
        ) => EndGettingEntriesMethod(
        );
        private static readonly EndGettingEntriesDelegate EndGettingEntriesMethod = ReflectionWrapperUtil.CreateStaticMethod<EndGettingEntriesDelegate>(
            BackedType,
            "EndGettingEntries",
            new global::System.Type[] {
            });
        delegate void EndGettingEntriesDelegate(
        );
        public static int GetCount(
        ) => GetCountMethod(
        );
        private static readonly GetCountDelegate GetCountMethod = ReflectionWrapperUtil.CreateStaticMethod<GetCountDelegate>(
            BackedType,
            "GetCount",
            new global::System.Type[] {
            });
        delegate int GetCountDelegate(
        );
        public static void GetCountsByType(
            ref int errorCount, 
            ref int warningCount, 
            ref int logCount
        ) => GetCountsByTypeMethod(
            ref errorCount, 
            ref warningCount, 
            ref logCount
        );
        private static readonly GetCountsByTypeDelegate GetCountsByTypeMethod = ReflectionWrapperUtil.CreateStaticMethod<GetCountsByTypeDelegate>(
            BackedType,
            "GetCountsByType",
            new global::System.Type[] {
                typeof(int).MakeByRefType(), 
                typeof(int).MakeByRefType(), 
                typeof(int).MakeByRefType(), 
            });
        delegate void GetCountsByTypeDelegate(
                ref int errorCount, 
                ref int warningCount, 
                ref int logCount
        );
        public static void GetLinesAndModeFromEntryInternal(
            int row, 
            int numberOfLines, 
            ref int mask, 
            ref string outString
        ) => GetLinesAndModeFromEntryInternalMethod(
            row, 
            numberOfLines, 
            ref mask, 
            ref outString
        );
        private static readonly GetLinesAndModeFromEntryInternalDelegate GetLinesAndModeFromEntryInternalMethod = ReflectionWrapperUtil.CreateStaticMethod<GetLinesAndModeFromEntryInternalDelegate>(
            BackedType,
            "GetLinesAndModeFromEntryInternal",
            new global::System.Type[] {
                typeof(int), 
                typeof(int), 
                typeof(int).MakeByRefType(), 
                typeof(string).MakeByRefType(), 
            });
        delegate void GetLinesAndModeFromEntryInternalDelegate(
                int row, 
                int numberOfLines, 
                ref int mask, 
                ref string outString
        );
        public static bool GetEntryInternal(
            int row, 
            LogEntry outputEntry
        ) => GetEntryInternalMethod(
            row, 
            outputEntry
        );
        private static readonly GetEntryInternalDelegate GetEntryInternalMethod = ReflectionWrapperUtil.CreateStaticMethod<GetEntryInternalDelegate>(
            BackedType,
            "GetEntryInternal",
            new global::System.Type[] {
                typeof(int), 
                typeof(LogEntry), 
            });
        delegate bool GetEntryInternalDelegate(
                int row, 
                LogEntry outputEntry
        );
        public static int GetEntryCount(
            int row
        ) => GetEntryCountMethod(
            row
        );
        private static readonly GetEntryCountDelegate GetEntryCountMethod = ReflectionWrapperUtil.CreateStaticMethod<GetEntryCountDelegate>(
            BackedType,
            "GetEntryCount",
            new global::System.Type[] {
                typeof(int), 
            });
        delegate int GetEntryCountDelegate(
                int row
        );
        public static void Clear(
        ) => ClearMethod(
        );
        private static readonly ClearDelegate ClearMethod = ReflectionWrapperUtil.CreateStaticMethod<ClearDelegate>(
            BackedType,
            "Clear",
            new global::System.Type[] {
            });
        delegate void ClearDelegate(
        );
        public static int GetStatusViewErrorIndex(
        ) => GetStatusViewErrorIndexMethod(
        );
        private static readonly GetStatusViewErrorIndexDelegate GetStatusViewErrorIndexMethod = ReflectionWrapperUtil.CreateStaticMethod<GetStatusViewErrorIndexDelegate>(
            BackedType,
            "GetStatusViewErrorIndex",
            new global::System.Type[] {
            });
        delegate int GetStatusViewErrorIndexDelegate(
        );
        public static void ClickStatusBar(
            int count
        ) => ClickStatusBarMethod(
            count
        );
        private static readonly ClickStatusBarDelegate ClickStatusBarMethod = ReflectionWrapperUtil.CreateStaticMethod<ClickStatusBarDelegate>(
            BackedType,
            "ClickStatusBar",
            new global::System.Type[] {
                typeof(int), 
            });
        delegate void ClickStatusBarDelegate(
                int count
        );
        public static void AddMessageWithDoubleClickCallback(
            LogEntry outputEntry
        ) => AddMessageWithDoubleClickCallbackMethod(
            outputEntry
        );
        private static readonly AddMessageWithDoubleClickCallbackDelegate AddMessageWithDoubleClickCallbackMethod = ReflectionWrapperUtil.CreateStaticMethod<AddMessageWithDoubleClickCallbackDelegate>(
            BackedType,
            "AddMessageWithDoubleClickCallback",
            new global::System.Type[] {
                typeof(LogEntry), 
            });
        delegate void AddMessageWithDoubleClickCallbackDelegate(
                LogEntry outputEntry
        );
    }
    partial struct LogEntry
    {
        public string message
        {
            get => messageGetter(BackedValue);
            set => messageSetter(BackedValue, value);
        }
        private static readonly messageGetterDelegate messageGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<messageGetterDelegate>(BackedType, typeof(string), "message");
        private static readonly messageSetterDelegate messageSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<messageSetterDelegate>(BackedType, typeof(string), "message");
        delegate string messageGetterDelegate(object self);
        delegate void messageSetterDelegate(object self, string value);
        public string file
        {
            get => fileGetter(BackedValue);
            set => fileSetter(BackedValue, value);
        }
        private static readonly fileGetterDelegate fileGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<fileGetterDelegate>(BackedType, typeof(string), "file");
        private static readonly fileSetterDelegate fileSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<fileSetterDelegate>(BackedType, typeof(string), "file");
        delegate string fileGetterDelegate(object self);
        delegate void fileSetterDelegate(object self, string value);
        public int line
        {
            get => lineGetter(BackedValue);
            set => lineSetter(BackedValue, value);
        }
        private static readonly lineGetterDelegate lineGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<lineGetterDelegate>(BackedType, typeof(int), "line");
        private static readonly lineSetterDelegate lineSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<lineSetterDelegate>(BackedType, typeof(int), "line");
        delegate int lineGetterDelegate(object self);
        delegate void lineSetterDelegate(object self, int value);
        public int column
        {
            get => columnGetter(BackedValue);
            set => columnSetter(BackedValue, value);
        }
        private static readonly columnGetterDelegate columnGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<columnGetterDelegate>(BackedType, typeof(int), "column");
        private static readonly columnSetterDelegate columnSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<columnSetterDelegate>(BackedType, typeof(int), "column");
        delegate int columnGetterDelegate(object self);
        delegate void columnSetterDelegate(object self, int value);
        public int mode
        {
            get => modeGetter(BackedValue);
            set => modeSetter(BackedValue, value);
        }
        private static readonly modeGetterDelegate modeGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<modeGetterDelegate>(BackedType, typeof(int), "mode");
        private static readonly modeSetterDelegate modeSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<modeSetterDelegate>(BackedType, typeof(int), "mode");
        delegate int modeGetterDelegate(object self);
        delegate void modeSetterDelegate(object self, int value);
        public int instanceID
        {
            get => instanceIDGetter(BackedValue);
            set => instanceIDSetter(BackedValue, value);
        }
        private static readonly instanceIDGetterDelegate instanceIDGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<instanceIDGetterDelegate>(BackedType, typeof(int), "instanceID");
        private static readonly instanceIDSetterDelegate instanceIDSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<instanceIDSetterDelegate>(BackedType, typeof(int), "instanceID");
        delegate int instanceIDGetterDelegate(object self);
        delegate void instanceIDSetterDelegate(object self, int value);
        public int identifier
        {
            get => identifierGetter(BackedValue);
            set => identifierSetter(BackedValue, value);
        }
        private static readonly identifierGetterDelegate identifierGetter =
            ReflectionWrapperUtil.CreateInstanceFieldGetter<identifierGetterDelegate>(BackedType, typeof(int), "identifier");
        private static readonly identifierSetterDelegate identifierSetter =
            ReflectionWrapperUtil.CreateInstanceFieldSetter<identifierSetterDelegate>(BackedType, typeof(int), "identifier");
        delegate int identifierGetterDelegate(object self);
        delegate void identifierSetterDelegate(object self, int value);
    }
}
