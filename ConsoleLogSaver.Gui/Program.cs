using System.Diagnostics;
using Anatawa12.ConsoleLogSaver;
using Anatawa12.ConsoleLogSaver.Gui;
using Mono.Debugger.Soft;

sealed class MainWindow : Form
{
    private ConsoleLogSaver _saver = new();
    public ListView UnityInstances { get; }

    public LinkLabel VersionLabel { get; set; }

    // ReSharper disable once MemberInitializerValueIgnored
    public Button[] SelectionRequiredButtons { get; } = Array.Empty<Button>();

    [STAThread]
    public static void Main()
    {
        Application.Run(new MainWindow());
    }

    public MainWindow()
    {
        FormBorderStyle = FormBorderStyle.Fixed3D;
        Width = 400;
        Height = 400;
        Text = "Console Log Saver";

        UnityInstances = AddControl(70, new ListView
        {
            View = View.Details,
            Scrollable = true,
            MultiSelect = false,
            FullRowSelect = true,
            Columns =
            {
                new ColumnHeader { Text = "PID", Width = -2 },
                new ColumnHeader { Text = "Project Path", Width = -2 },
            },
        });
        UnityInstances.SelectedIndexChanged += (_, _) => SetButtonEnabled();

        AddButton(Localization.UpdateRunningUnityList, (_, _) => ReloadUnity());

        VersionLabel = AddControl(15, new LinkLabel
        {
            Text = string.Format(Localization.VersionAndCheckingForUpdates, CheckForUpdate.CurrentVersion),
            LinkArea = new LinkArea(0, 0),
        });

        AddControl(new Label { Text = Localization.SecuritySettings }, 15);

        AddControl(new CheckBox { Text = Localization.UnityVersion, Checked = true, Enabled = false }, 15);

        void FieldCheckBox(string text, bool @checked, Action<bool> setter)
        {
            var box = AddControl(new CheckBox { Text = text, Checked = @checked }, 15);
            box.CheckedChanged += (_, _) => setter(box.Checked);
        }

        FieldCheckBox(Localization.HideOSInfo, _saver.HideOsInfo, v => _saver.HideOsInfo = v);
        FieldCheckBox(Localization.HideUserName, _saver.HideUserName, v => _saver.HideUserName = v);
        FieldCheckBox(Localization.HideUserHome, _saver.HideUserHome, v => _saver.HideUserHome = v);

        SelectionRequiredButtons = new[]
        {
            AddButton(Localization.SaveToFile, SaveToFile, false),
            AddButton(Localization.CopyToClipboard, CopyToClipboard, false),
        };

        ReloadUnity();
        StartCheckForUpdates();
    }

    private void SetButtonEnabled()
    {
        var enabled = UnityInstances.SelectedItems.Count != 0;

        foreach (var button in SelectionRequiredButtons)
            button.Enabled = enabled;
    }

    private async void StartCheckForUpdates()
    {
        var newVersion = await CheckForUpdate.Check();
        if (newVersion is {} tuple)
        {
            var (isOutdated, version) = tuple;
            if (isOutdated)
            {
                var format = string.Format(Localization.VersionAndNewFound, CheckForUpdate.CurrentVersion, version);
                var parts = format.Split('$', 3);
                VersionLabel.Text = parts[0] + parts[1] + parts[2];
                VersionLabel.LinkArea = new LinkArea(parts[0].Length, parts[1].Length);
                VersionLabel.LinkClicked += (_, _) =>
                {
                    Process.Start(new ProcessStartInfo("https://github.com/anatawa12/ConsoleLogSaver")
                    {
                        UseShellExecute = true
                    });
                };
            }
            else
            {
                VersionLabel.Text = string.Format(Localization.VersionAndLatest, CheckForUpdate.CurrentVersion);
            }
        }
        else
        {
            VersionLabel.Text = string.Format(Localization.VersionAndFailed, CheckForUpdate.CurrentVersion);
        }
    }

    private int _yPosition = 10;

    private Button AddButton(string text, EventHandler handler, bool enabled = true)
    {
        var generate = AddControl(new Button(), 30);
        generate.Text = text;
        generate.Click += handler;
        generate.Enabled = enabled;
        return generate;
    }

    private T AddControl<T>(T control, int height) where T : Control => AddControl(height, control);

    private T AddControl<T>(int height, T control) where T : Control
    {
        const int width = 360;
        control.Location = new Point(10, _yPosition);
        control.Size = new Size(width, height);
        Controls.Add(control);
        _yPosition += height + 10;
        return control;
    }

    private void DisconnectAll()
    {
        foreach (ListViewItem item in UnityInstances.Items)
        {
            try
            {
                (item as UnitySessionItem)?.Session?.Dispose();
            }
            catch
            {
                // ignored
            }
        }
    }

    private async void ReloadUnity()
    {
        DisconnectAll();

        UnityInstances.Items.Clear();
        SetButtonEnabled();

        var processes = await DebuggerSession.ConnectAllUnityProcesses(TimeSpan.FromSeconds(2));

        UnityInstances.Items.AddRange(processes.Select(s => new UnitySessionItem(s)).ToArray<ListViewItem>());
        SetButtonEnabled();
    }

    protected override void OnFormClosed(FormClosedEventArgs e)
    {
        base.OnFormClosed(e);
        DisconnectAll();
    }

    private async Task<ConsoleLogFileV1?> CollectData()
    {
        try
        {
            var item = (UnitySessionItem)UnityInstances.SelectedItems[0];
            return await _saver.Collect(item.Session);
        }
        catch (VMDisconnectedException)
        {
            MessageBox.Show("The Unity Process exited.", "ERROR");
            return null;
        }
    }

    private async void SaveToFile(object? sender, EventArgs e)
    {
        var openFile = new SaveFileDialog
        {
            Title = "Save To File",
            FileName = "logfile.txt",
            Filter = "Text files (*.txt)|*.txt",
        };

        if (openFile.ShowDialog() == DialogResult.OK)
        {
            var collect = await CollectData();
            if (collect == null) return;
            var asText = LogFileWriter.WriteToString(collect);
            using var writer = new StreamWriter(openFile.OpenFile());
            await writer.WriteAsync(asText);
        }
    }

    private async void CopyToClipboard(object? sender, EventArgs e)
    {
        var collect = await CollectData();
        if (collect == null) return;
        Clipboard.SetText(LogFileWriter.WriteToString(collect));
        MessageBox.Show("Copied", "Copied!");
    }

    class UnitySessionItem : ListViewItem
    {
        public UnitySessionItem(DebuggerSession session) : base(new[] { session.Pid.ToString(), session.ProjectRoot })
        {
            Session = session;
        }

        public DebuggerSession Session { get; }
    }
}
