using System;
using System.IO;
using System.Linq;
using System.Linq.Expressions;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEngine;

namespace anatawa12.gists
{
    internal class ConsoleLogSaverSetting : EditorWindow
    {
        public bool hideOsInfo = false;
        public bool hideUserName = true;
        public bool hideUserHome = true;

        private SerializedObject _serializedObject;
        private SerializedProperty _hideOsInfoProp;
        private SerializedProperty _hideUserNameProp;
        private SerializedProperty _hideUserHomeProp;

        private void OnEnable()
        {
            _serializedObject = new SerializedObject(this);
            _hideOsInfoProp = _serializedObject.FindProperty(nameof(hideOsInfo));
            _hideUserNameProp = _serializedObject.FindProperty(nameof(hideUserName));
            _hideUserHomeProp = _serializedObject.FindProperty(nameof(hideUserHome));
        }

        private void OnGUI()
        {
            GUILayout.Label("ConsoleLogSaver");
            EditorGUILayout.LabelField("Security Setting");
            _serializedObject.Update();
            EditorGUI.BeginDisabledGroup(true);
            EditorGUILayout.Toggle("Unity Version (required)", true);
            EditorGUI.EndDisabledGroup();
            EditorGUILayout.PropertyField(_hideOsInfoProp);
            EditorGUILayout.PropertyField(_hideUserNameProp);
            EditorGUILayout.PropertyField(_hideUserHomeProp);
            _serializedObject.ApplyModifiedProperties();
            GUILayout.BeginHorizontal();
            /*
            if (GUILayout.Button("Upload & get link"))
            {
                EditorUtility.DisplayDialog("Error", "Not Implemented yet", "OK");
            }
            */
            if (GUILayout.Button("Save to File"))
            {
                var path = EditorUtility.SaveFilePanel("Save logs to File",
                    ".", "logfile.txt", "txt");
                if (!string.IsNullOrEmpty(path))
                {
                    File.WriteAllText(path, Generate(), Encoding.UTF8);
                }
            }
            if (GUILayout.Button("Copy to Clipboard"))
            {
                GUIUtility.systemCopyBuffer = Generate();
                EditorUtility.DisplayDialog("Copied!", "Copied log info to clipboard done!", "OK");
            }
            GUILayout.EndHorizontal();
        }

        private string Generate()
        {
            InitRegex();
            var backupFlags = LogEntries.consoleFlags;
            LogEntries.SetConsoleFlag((int) ConsoleFlags.Collapse, false);
            LogEntries.SetConsoleFlag((int) ConsoleFlags.LogLevelLog, true);
            LogEntries.SetConsoleFlag((int) ConsoleFlags.LogLevelError, true);
            LogEntries.SetConsoleFlag((int) ConsoleFlags.LogLevelWarning, true);

            var separator = 
                "================" 
                + GUID.Generate().ToString().Replace("-", "") 
                + "================";
            var builder = new StringBuilder();
            // header
            builder.Append("ConsoleLogSaverData/1.0\n");
            builder.Append("Separator: ").Append(separator).Append('\n');
            builder.Append("Unity-Version: ").Append(Application.unityVersion).Append('\n');
            builder.Append("Build-Target: ").Append(EditorUserBuildSettings.activeBuildTarget).Append('\n');
            if (!hideOsInfo) builder.Append("Editor-Platform: ").Append(RuntimeInformation.OSDescription).Append('\n');
            if (hideUserName) builder.Append("Hidden-Data: user-name\n");
            if (hideUserHome) builder.Append("Hidden-Data: user-home\n");
            builder.Append('\n');
            builder.Append(separator).Append('\n');

            using (var scope = new GettingLogEntriesScope(0))
            {
                var entry = LogEntry.New();
                for (var i = 0; i < scope.TotalRows; i++)
                {
                    LogEntries.GetEntryInternal(i, entry);
                    var mode = entry.mode;
                    builder.Append("Content: log-element\n");
                    builder.Append("Mode: ").Append((Mode)mode).Append('\n');
                    builder.Append("Mode-Raw: ").Append($"{mode:x08}").Append('\n');
                    builder.Append('\n');
                    builder.Append(ReplaceMessage(entry.message));
                    builder.Append(separator).Append('\n');
                }
            }

            LogEntries.consoleFlags = backupFlags;
            
            return builder.ToString(); 
        }

        private Regex _homePatternRegex;
        private Regex _namePatternRegex;

        private void InitRegex()
        {
            _homePatternRegex = null;
            _namePatternRegex = null;
            if (hideUserHome)
            {
                var homePath = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
                var pathComponents = homePath.Split('/', '\\');
                _homePatternRegex = new Regex(string.Join("[\\\\/]", pathComponents.Select(Regex.Escape)),
                    RegexOptions.IgnoreCase);
            }
            
            if (hideUserName)
            {
                _namePatternRegex = new Regex(Regex.Escape(Environment.UserName), RegexOptions.IgnoreCase);
            }
        }

        private string ReplaceMessage(string str)
        {
            if (_namePatternRegex != null)
                str = _namePatternRegex.Replace(str, "${user-name}");
            if (_homePatternRegex != null)
                str = _homePatternRegex.Replace(str, "${user-home}");
            return str;
        }

        [MenuItem("Tools/Console Log Saver")]
        private static void HideName()
        {
            GetWindowWithRect<ConsoleLogSaverSetting>(
                    new Rect(0, 0, 300, 300),
                    true, "ConsoleLogSaverSetting")
                .Show();
        }
    }

    [Flags]
    internal enum Mode
    {
        Error = 1 << 0,
        Assert = 1 << 1,
        Log = 1 << 2,
        Fatal = 1 << 4,
        DontPreprocessCondition = 1 << 5,
        AssetImportError = 1 << 6,
        AssetImportWarning = 1 << 7,
        ScriptingError = 1 << 8,
        ScriptingWarning = 1 << 9,
        ScriptingLog = 1 << 10,
        ScriptCompileError = 1 << 11,
        ScriptCompileWarning = 1 << 12,
        StickyError = 1 << 13,
        MayIgnoreLineNumber = 1 << 14,
        ReportBug = 1 << 15,
        DisplayPreviousErrorInStatusBar = 1 << 16,
        ScriptingException = 1 << 17,
        DontExtractStacktrace = 1 << 18,
        ShouldClearOnPlay = 1 << 19,
        GraphCompileError = 1 << 20,
        ScriptingAssertion = 1 << 21,
        VisualScriptingError = 1 << 22
    }

    [Flags]
    enum ConsoleFlags
    {
        Collapse = 1 << 0,
        ClearOnPlay = 1 << 1,
        ErrorPause = 1 << 2,
        Verbose = 1 << 3,
        StopForAssert = 1 << 4,
        StopForError = 1 << 5,
        Autoscroll = 1 << 6,
        LogLevelLog = 1 << 7,
        LogLevelWarning = 1 << 8,
        LogLevelError = 1 << 9,
        ShowTimestamp = 1 << 10,
        ClearOnBuild = 1 << 11,
        ClearOnRecompile = 1 << 12,
        UseMonospaceFont = 1 << 13,
        StripLoggingCallstack = 1 << 14,
    }

    internal struct GettingLogEntriesScope : IDisposable
    {
        private bool _disposed;
        public readonly int TotalRows;

        public GettingLogEntriesScope(int holder)
        {
            _disposed = false;
            TotalRows = LogEntries.StartGettingEntries();
        }

        public void Dispose()
        {
            if (_disposed)
                return;
            LogEntries.EndGettingEntries();
            _disposed = true;
        }
    }

    static class ReflectionWrapperUtil
    {
        public static T CreateStaticMethod<T>(Type type, string name, Type[] parameters)
        {
            var actualType = parameters.Any(x => x == typeof(LogEntry))
                ? parameters.Select(x => x == typeof(LogEntry) ? LogEntry.BackedType : x).ToArray()
                : parameters;
            var m = type.GetMethod(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static, null,
                        actualType, null)
                    ?? throw new Exception("method not found");
            return CreateStaticMethod<T>(m, parameters);
        }

        private static T CreateStaticMethod<T>(MethodInfo m, Type[] parameters)
        {
            var args = parameters.Select((t, i) => Expression.Parameter(t, $"param{i}")).ToArray();
            var argsValues = args.Select((arg, i) =>
                    arg.Type == typeof(LogEntry)
                        ? (Expression)Expression.Convert(Expression.Field(arg, "BackedValue"), LogEntry.BackedType)
                        : arg)
                .ToArray();
            var call = Expression.Call(null, m, argsValues.Cast<Expression>());
            return Expression.Lambda<T>(call, args).Compile();
        }

        public static T CreateInstanceFieldGetter<T>(Type backedType, Type type, string name)
        {
            var self = Expression.Parameter(typeof(object), "self");
            var f = backedType.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)
                ?? throw new Exception("field not found");
            return Expression.Lambda<T>(Expression.Field(Expression.Convert(self, backedType), f), self)
                .Compile();
        }

        public static T CreateInstanceFieldSetter<T>(Type backedType, Type type, string name)
        {
            var self = Expression.Parameter(typeof(object), "self");
            var value = Expression.Parameter(type, "value");
            var f = backedType.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)
                ?? throw new Exception("field not found");
            return Expression.Lambda<T>(
                Expression.Assign(Expression.Field(Expression.Convert(self, backedType), f), value), 
                self, value)
                .Compile();
        }

        public static T CreateStaticPropertyGetter<T>(Type backedType, Type type, string name)
        {
            var prop = backedType.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)
                       ?? throw new Exception("property not found");
            var method = prop.GetMethod;
            return CreateStaticMethod<T>(method, Type.EmptyTypes);
        }

        public static T CreateStaticPropertySetter<T>(Type backedType, Type type, string name)
        {
            var prop = backedType.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)
                       ?? throw new Exception("property not found");
            var method = prop.SetMethod;
            return CreateStaticMethod<T>(method, new []{type});
        }
    }

    // this class is reflection wrapper class of UnityEditor.LogEntries.
    // used to pull log messages from Cpp side to mono window
    // All functions marked internal may not be called unless you call StartGettingEntries and EndGettingEntries
    static partial class LogEntries
    {
        public static Type BackedType => 
            _backedType ?? (_backedType = typeof(Editor).Assembly.GetType("UnityEditor.LogEntries")); 
        private static Type _backedType;
    }

    readonly partial struct LogEntry
    {
        public static Type BackedType => 
            _backedType ?? (_backedType = typeof(Editor).Assembly.GetType("UnityEditor.LogEntry")); 
        private static Type _backedType;
        internal readonly object BackedValue;

        private LogEntry(object backedValue)
        {
            BackedValue = backedValue;
        }

        public static LogEntry New()
        {
            return new LogEntry(Activator.CreateInstance(BackedType));
        }
    }
}
