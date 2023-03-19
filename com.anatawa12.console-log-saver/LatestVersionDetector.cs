using System;
using System.Linq;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using CustomLocalization4EditorExtension;
using JetBrains.Annotations;
using UnityEditor;
using UnityEngine;

namespace Anatawa12.ConsoleLogSaver
{
    static class LatestVersionDetector
    {
        [CanBeNull] public static readonly string CurrentVersion = GetCurrentVersion();
        [CanBeNull] private static Task _latestVersionTask;
        private static bool _isOutdated = false;
        private static DateTime _lastCheckDate = DateTime.MinValue;

        public static void ShowUpdateNoticeIfNeeded([CanBeNull] Action repaint)
        {
            if (CurrentVersion == null) return;
            if (_isOutdated)
            {
                DoDrawUpdateNotice();
                return;
            }
            // update check
            if (_lastCheckDate < DateTime.UtcNow - TimeSpan.FromHours(6))
            {
                var callbackSync = SynchronizationContext.Current;
                Task.Run(async () =>
                {
                    var latestVersion = await GetLatestVersion(repaint).ConfigureAwait(false);

                    var currentParsed = ParseVersion(CurrentVersion);
                    var latestParsed = ParseVersion(latestVersion);

                    _isOutdated = IsOutdated(currentParsed, latestParsed);

                    if (repaint != null)
                        callbackSync.Post(action => ((Action)action)(), repaint);
                });
                _lastCheckDate = DateTime.UtcNow;
            }
        }

        private static bool IsOutdated(Version currentParsed, Version latestParsed)
        {
            if (currentParsed.Major < latestParsed.Major) return true;
            if (currentParsed.Major > latestParsed.Major) return false;
            if (currentParsed.Minor < latestParsed.Minor) return true;
            if (currentParsed.Minor > latestParsed.Minor) return false;
            if (currentParsed.Patch < latestParsed.Patch) return true;
            if (currentParsed.Patch > latestParsed.Patch) return false;
            if (currentParsed.IsBeta) return true;
            return false;
        }

        private static Version ParseVersion([NotNull] string version)
        {
            var parts = version.Split(new[] { '-' }, 2);
            var numbers = parts[0].Split('.');
            int maj = int.Parse(numbers[0]);
            int min = numbers.Length > 1 ? int.Parse(numbers[1]) : 0;
            int pat = numbers.Length > 2 ? int.Parse(numbers[2]) : 0;
            return new Version(maj, min, pat, parts.Length > 1);
        }

        readonly struct Version
        {
            public readonly int Major;
            public readonly int Minor;
            public readonly int Patch;
            public readonly bool IsBeta;

            public Version(int major, int minor, int patch, bool isBeta)
            {
                Major = major;
                Minor = minor;
                Patch = patch;
                IsBeta = isBeta;
            }
        }

        private static void DoDrawUpdateNotice()
        {
            EditorGUILayout.HelpBox(CL4EE.Tr("updateNotice:box-message"), MessageType.Info);
            if (GUILayout.Button(CL4EE.Tr("updateNotice:go-button")))
                Application.OpenURL("https://github.com/anatawa12/ConsoleLogSaver");
        }

        private static async Task<string> GetLatestVersion([CanBeNull] Action repaint)
        {
            var url = "https://github.com/anatawa12/ConsoleLogSaver/raw/master/latest.txt";
            using (var client = new HttpClient())
            {
                client.DefaultRequestHeaders.TryAddWithoutValidation("User-Agent",
                    $"ConsoleLogSaver-update-checker/{CurrentVersion} (https://github.com/anatawa12/ConsoleLogSaver)");
                var response = await client.GetAsync(url).ConfigureAwait(false);
                var content = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
                var lines = content.Split(new[] { "\r\n", "\n" }, 2, StringSplitOptions.None);
                return lines[0];
            }
        }

        [CanBeNull]
        private static string GetCurrentVersion()
        {
            var path = AssetDatabase.GUIDToAssetPath("472e300033394b98b892468ed5929e6f");
            var file = AssetDatabase.LoadAssetAtPath<TextAsset>(path);
            if (file == null) return null;
            var text = file.text.Split('\n');
            var versionLine = text.FirstOrDefault(x => x.StartsWith("version=", StringComparison.Ordinal));
            if (versionLine == null) return null;
            return versionLine.Substring("version=".Length).Trim();
        }
    }
}
