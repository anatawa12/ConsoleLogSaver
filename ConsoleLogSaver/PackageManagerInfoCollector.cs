using System.Text.Json;
using System.Text.Json.Serialization;

namespace Anatawa12.ConsoleLogSaver;

static partial class PackageManagerInfoCollector
{
    // returns list of locked dependencies listed on vpm-manifest.json
    public static IEnumerable<(string package, string version)> VpmLockedPackages(string projectRoot)
    {
        try
        {
            var vpmManifestJson = File.ReadAllText(Path.Join(projectRoot, "Packages", "vpm-manifest.json"));
            var manifest = JsonSerializer.Deserialize(vpmManifestJson, SourceGenerationContext.Default.VpmManifest)
                           ?? throw new InvalidOperationException();
            return manifest.Locked
                .Where(x => x.Value.Version != null)
                .Select(x => (x.Key, x.Value.Version!));
        }
        catch
        {
            return Array.Empty<(string, string)>();
        }
    }

    // returns list of locked dependencies listed on packages-lock.json
    public static IEnumerable<(string package, UpmDependencyType type, string version)> UpmLockedPackages(
        string projectRoot)
    {
        try
        {
            var vpmManifestJson = File.ReadAllText(Path.Join(projectRoot, "Packages", "packages-lock.json"));
            var manifest = JsonSerializer.Deserialize(vpmManifestJson, SourceGenerationContext.Default.UpmLockFile)
                           ?? throw new InvalidOperationException();
            return manifest.Dependencies
                .Where(x => x.Value.Version != null)
                .Select(x =>
                {
                    var version = x.Value.Version!;
                    var type = DetectUpmDependencyType(version);
                    return (x.Key, type, version);
                });
        }
        catch
        {
            return Array.Empty<(string, UpmDependencyType, string)>();
        }
    }

    private static UpmDependencyType DetectUpmDependencyType(string version)
    {
        if (version.StartsWith("file://", StringComparison.Ordinal)
            || version.Contains(".git")
            || version.StartsWith("git+", StringComparison.Ordinal))
        {
            // it's some git URLs
            if (version.StartsWith("git+", StringComparison.Ordinal))
                version = version.Substring("git+".Length);

            if (version.StartsWith("https:", StringComparison.Ordinal))
                return UpmDependencyType.HttpsGit;
            if (version.StartsWith("ssh:", StringComparison.Ordinal))
                return UpmDependencyType.SshGit;
            if (version.StartsWith("file:", StringComparison.Ordinal))
                return UpmDependencyType.FileGit;
            if (version.StartsWith("git:", StringComparison.Ordinal))
                return UpmDependencyType.GitGit;
        }

        if (version.StartsWith("file:", StringComparison.Ordinal))
        {
            // it's some file URLs
            var path = version.Substring("file:".Length);
            if (Path.IsPathRooted(path))
                return UpmDependencyType.FileAbsolute;
            else
                return UpmDependencyType.FileRelative;
        }

        return UpmDependencyType.Upm;
    }

    [JsonSourceGenerationOptions(WriteIndented = true)]
    [JsonSerializable(typeof(VpmManifest))]
    [JsonSerializable(typeof(UpmLockFile))]
    partial class SourceGenerationContext : JsonSerializerContext
    {
    }

    class VpmManifest
    {
        [JsonPropertyName("locked")]
        public IDictionary<string, VpmLockedDependency> Locked { get; set; } = new Dictionary<string, VpmLockedDependency>();
    }

    class VpmLockedDependency
    {
        [JsonPropertyName("version")]
        public string? Version { get; set; }
    }

    class UpmLockFile
    {
        [JsonPropertyName("dependencies")]
        public IDictionary<string, UpmLockedDependency> Dependencies { get; set; } = new Dictionary<string, UpmLockedDependency>();
    }

    class UpmLockedDependency
    {
        [JsonPropertyName("version")]
        public string? Version { get; set; }
    }
}

enum UpmDependencyType
{
    Upm,

    // gits
    HttpsGit,
    SshGit,
    FileGit,
    GitGit,

    // locals, including tgz
    FileRelative,
    FileAbsolute,
}
