using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

namespace Anatawa12.ConsoleLogSaver
{
    public class ConsoleLogFileV1
    {
        public int MinorVersion { get; }
        public IReadOnlyList<(string name, string value)> HeaderValues { get; }
        public IReadOnlyList<Section> Sections { get; }

        private ConsoleLogFileV1(Builder builder)
        {
            MinorVersion = builder.MinorVersion;
            HeaderValues = builder.HeaderValues.ToList();
            Sections = builder.Sections.ToList();
        }

        public class Builder
        {
            private FieldsBuilder _fields = new FieldsBuilder("Separator");
            private List<Section> _sections = new List<Section>();
            public int MinorVersion { get; }

            public IReadOnlyList<(string name, string value)> HeaderValues => _fields.Fields;
            public IReadOnlyList<Section> Sections => _sections;

            public Builder(int minorVersion)
            {
                MinorVersion = minorVersion;
            }

            public ConsoleLogFileV1 Build() => new ConsoleLogFileV1(this);

            public Builder AddField(string name, string value)
            {
                _fields.AddField(name, value);
                return this;
            }

            public Builder AddField(string name, string value, bool sanitizeValue)
            {
                _fields.AddField(name, value, sanitizeValue);
                return this;
            }

            public Builder AddSection(Section section)
            {
                _sections.Add(section);
                return this;
            }
        }
    }

    public class Section
    {
        public IReadOnlyList<(string name, string value)> Fields { get; }
        public string Content { get; }

        public string ContentType => Fields[0].name.Equals("Content", StringComparison.OrdinalIgnoreCase)
            ? Fields[0].value
            : throw new InvalidOperationException("this section is header section");

        private Section(Builder builder)
        {
            Fields = builder.Fields.ToList();
            Content = builder.Content.ToString();
        }

        public class Builder
        {
            private FieldsBuilder _fields = new FieldsBuilder("Content");
            public IReadOnlyList<(string name, string value)> Fields => _fields.Fields;
            public StringBuilder Content { get; } = new StringBuilder();

            public Section Build() => new Section(this);

            public Builder(string contentType)
            {
                if (!CheckContentType(contentType))
                    throw new ArgumentException("invalid contentType", nameof(contentType));
                _fields.ForceAddField("Content", contentType);
            }

            // for header section
            private Builder()
            {
            }

            // to avoid miss, separate to static method
            internal static Builder NewForHeadingSection() => new Builder();

            public Builder AddField(string name, string value)
            {
                _fields.AddField(name, value);
                return this;
            }

            public Builder AddField(string name, string value, bool sanitizeValue)
            {
                _fields.AddField(name, value, sanitizeValue);
                return this;
            }

            private bool CheckContentType(string name)
            {
                if (name.Length == 0) return false;
                const string allowedChars = "-._0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
                return name.All(c => allowedChars.Contains(c));
            }
        }
    }

    class FieldsBuilder
    {
        private List<(string name, string value)> _fields = new List<(string name, string value)>();
        private readonly HashSet<string> _disallowedNames;
        public IReadOnlyList<(string name, string value)> Fields => _fields;

        public FieldsBuilder(params string[] disallowedNames)
        {
            _disallowedNames = new HashSet<string>(disallowedNames.Select(x => x.ToLowerInvariant()));
        }

        public FieldsBuilder AddField(string name, string value) => AddField(name, value, true);

        public FieldsBuilder AddField(string name, string value, bool sanitizeValue)
        {
            if (_disallowedNames.Contains(name.ToLowerInvariant()))
                throw new ArgumentException($"disallowed field name: {name}", nameof(name));
            ForceAddField(name, value, sanitizeValue);
            return this;
        }

        public void ForceAddField(string name, string value, bool sanitizeValue = false)
        {
            if (!CheckFieldName(name))
                throw new ArgumentException("invalid field name", nameof(name));
            if (value.Contains('\r') || value.Contains('\n'))
            {
                if (!sanitizeValue)
                    throw new ArgumentException("invalid field value", nameof(value));
                value = string.Join(" ", value.Split(new[] {"\r\n", "\r", "\n"}, StringSplitOptions.None));
            }

            _fields.Add((name, value));
        }

        private bool CheckFieldName(string name)
        {
            if (name.Length == 0) return false;
            const string allowedChars = "!#$%&'*+-.^_`|~0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
            return name.All(c => allowedChars.Contains(c));
        }
    }

    public static class LogFileWriter
    {
        public static string WriteToString(ConsoleLogFileV1 file)
        {
            var builder = new StringBuilder();

            var separator = "================" + Guid.NewGuid().ToString("N") + "================";
            builder.AppendFile(separator, file);

            return builder.ToString();
        }

        private static void AppendFile(this StringBuilder builder, string separator, ConsoleLogFileV1 file)
        {
            builder.Append($"ConsoleLogSaverData/1.{file.MinorVersion}\n");
            builder.AppendFields(new[] {("Separator", separator)}.Concat(file.HeaderValues));
            builder.Append(separator).Append('\n');
            foreach (var section in file.Sections)
            {
                builder.AppendSection(section);
                builder.Append(separator).Append('\n');
            }
        }

        private static void AppendSection(this StringBuilder builder, Section section)
        {
            builder.AppendFields(section.Fields);
            builder.Append(section.Content);
        }

        private static void AppendFields(this StringBuilder builder, IEnumerable<(string name, string value)> fields)
        {
            foreach (var (name, value) in fields)
                builder.Append(name).Append(": ").Append(value).Append('\n');
            builder.Append('\n');
        }
    }
}
