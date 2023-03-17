using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Anatawa12.SimpleJson;

namespace Anatawa12.ConsoleLogSaver
{
    static class PackageManagerInfoCollector
    {
        // returns list of locked dependencies listed on vpm-manifest.json
        public static IEnumerable<(string package, string version)> VpmLockedPackages()
        {
            JsonObj locked;
            try
            {
                var vpmManifest = new JsonParser(File.ReadAllText("Packages/vpm-manifest.json")).Parse(JsonType.Obj);
                locked = vpmManifest.Get("locked", JsonType.Obj);
            }
            catch
            {
                yield break;
            }

            foreach (var (package, value) in locked)
            {
                if (!(value is JsonObj lockedInfo)) continue;
                var version = lockedInfo.Get("version", JsonType.String);
                yield return (package, version);
            }
        }

        // returns list of locked dependencies listed on packages-lock.json
        public static IEnumerable<(string package, UpmDependencyType type, string version)> UpmLockedPackages()
        {
            JsonObj dependencies;
            try
            {
                var vpmManifest = new JsonParser(File.ReadAllText("Packages/packages-lock.json")).Parse(JsonType.Obj);
                dependencies = vpmManifest.Get("dependencies", JsonType.Obj);
            }
            catch
            {
                yield break;
            }

            foreach (var (package, value) in dependencies)
            {
                if (!(value is JsonObj lockedInfo)) continue;
                var version = lockedInfo.Get("version", JsonType.String);
                var type = DetectUpmDependencyType(version);
                yield return (package, type, version);
            }
        }

        private static UpmDependencyType DetectUpmDependencyType(string version)
        {
            if (version.StartsWith("file://") || version.Contains(".git") || version.StartsWith("git+"))
            {
                // it's some git URLs
                if (version.StartsWith("git+"))
                    version = version.Substring("git+".Length);

                if (version.StartsWith("https:"))
                    return UpmDependencyType.HttpsGit;
                if (version.StartsWith("ssh:"))
                    return UpmDependencyType.SshGit;
                if (version.StartsWith("file:"))
                    return UpmDependencyType.FileGit;
                if (version.StartsWith("git:"))
                    return UpmDependencyType.GitGit;
            }
            if (version.StartsWith("file:"))
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
}
