// generated
// ReSharper disable InconsistentNaming
// ReSharper disable RedundantNameQualifier
namespace anatawa12.gists
{
    static partial class LogEntries
    {
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
        public static int consoleFlags
        {
            get => consoleFlagsGetter();
            set => consoleFlagsSetter(value);
        }
        private static readonly consoleFlagsGetterDelegate consoleFlagsGetter =
            ReflectionWrapperUtil.CreateStaticPropertyGetter<consoleFlagsGetterDelegate>(BackedType, typeof(int), "consoleFlags");
        private static readonly consoleFlagsSetterDelegate consoleFlagsSetter =
            ReflectionWrapperUtil.CreateStaticPropertySetter<consoleFlagsSetterDelegate>(BackedType, typeof(int), "consoleFlags");
        delegate int consoleFlagsGetterDelegate();
        delegate void consoleFlagsSetterDelegate(int value);
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
    }
}
