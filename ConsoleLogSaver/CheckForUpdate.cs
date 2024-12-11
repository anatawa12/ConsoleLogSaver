using System.Diagnostics;
using System.Reflection;

namespace Anatawa12.ConsoleLogSaver;

public class CheckForUpdate
{
    private static string? _currentVersion;
    public static string CurrentVersion
    {
        get
        {
            if (_currentVersion != null) return _currentVersion;
            var version = System.Reflection.Assembly.GetExecutingAssembly().GetName().Version;
            Debug.Assert(version != null, nameof(version) + " != null");
            var version2 = Assembly.GetExecutingAssembly().GetCustomAttribute<AssemblyInformationalVersionAttribute>()!.InformationalVersion;
            var commit = version2.Contains('+') ? version2.Split('+')[1][..8] : "unknown";
            return _currentVersion = $"{version.Major}.{version.Minor}.{version.Build}-StacktraceSaver+{commit}";;
        }
    }

    public static async Task<(bool, string)?> Check()
    {
        try
        {
            var currentParsed = ParseVersion(CurrentVersion);

            const string url = "https://github.com/anatawa12/ConsoleLogSaver/raw/master/latest.txt";
            using var client = new HttpClient();
            client.DefaultRequestHeaders.TryAddWithoutValidation("User-Agent",
                $"ConsoleLogSaver-update-checker/{CurrentVersion} (https://github.com/anatawa12/ConsoleLogSaver)");
            var response = await client.GetAsync(url).ConfigureAwait(false);
            var content = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
            var lines = content.Split(new[] { "\r\n", "\n" }, 2, StringSplitOptions.None);
            var latest = lines[0];
            var latestParsed = ParseVersion(lines[0]);

            return (IsOutdated(currentParsed, latestParsed), latest);
        }
        catch
        {
            return null;
        }
    }

    private static Version ParseVersion(string version)
    {
        var parts = version.Split(new[] { '-' }, 2);
        var numbers = parts[0].Split('.');
        int maj = int.Parse(numbers[0]);
        int min = numbers.Length > 1 ? int.Parse(numbers[1]) : 0;
        int pat = numbers.Length > 2 ? int.Parse(numbers[2]) : 0;
        return new Version(maj, min, pat, parts.Length > 1);
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
}
