// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Text.Json;
using System.Text.Json.Serialization;
using VietFlow.Engine;

namespace VietFlow.Configuration;

[JsonSourceGenerationOptions(WriteIndented = true, UseStringEnumConverter = true)]
[JsonSerializable(typeof(VietFlowConfig))]
[JsonSerializable(typeof(Dictionary<string, string>))]
[JsonSerializable(typeof(List<string>))]
[JsonSerializable(typeof(InputMethod))]
[JsonSerializable(typeof(CharsetId))]
internal partial class VietFlowJsonContext : JsonSerializerContext
{
}

public sealed class VietFlowConfig
{
    public bool IsEnabled { get; set; } = true;
    public InputMethod Method { get; set; } = InputMethod.Telex;
    public CharsetId Charset { get; set; } = CharsetId.Unicode;
    public bool ModernStyle { get; set; } = false;
    public bool FreeMarking { get; set; } = true;
    public bool EnableMacro { get; set; } = true;
    public bool AllowMacroWhenDisabled { get; set; } = false;
    public bool SpellCheck { get; set; } = true;
    public bool AutoNonVnRestore { get; set; } = true;
    public bool FixShiftLag { get; set; } = true;
    public bool ConsonantShortcuts { get; set; } = false;
    public bool UseClipboardForUnicode { get; set; } = false;
    public bool StartWithWindows { get; set; } = false;
    public string ThemeMode { get; set; } = "System"; // "System", "Light", "Dark"
    public bool NotificationSound { get; set; } = true;
    public bool AutoCapitalize { get; set; } = false;
    public string TrayVnTop { get; set; } = "#EF4444";
    public string TrayVnBottom { get; set; } = "#B91C1C";
    public string TrayEnTop { get; set; } = "#2563EB";
    public string TrayEnBottom { get; set; } = "#1D4ED8";

    /// <summary>
    /// User-editable English/foreign keyword list (one per line). Empty = built-in defaults.
    /// </summary>
    public List<string> EnglishKeywords { get; set; } = new();

    /// <summary>
    /// Initial letters treated as non-Vietnamese; tones/marks are suppressed for these words.
    /// </summary>
    public string ForeignInitials { get; set; } = "fjwz";
    public string ToggleHotkey { get; set; } = "CtrlShift"; // "CtrlShift", "AltZ", "CtrlSpace", "ShiftSpace", "AltShift", "WinSpace", "CapsLock", "RightCtrl", "Custom"
    public string UiLanguage { get; set; } = "vi"; // "vi" or "en"
    public int CustomToggleVk { get; set; } = 0;
    public int CustomToggleModifiers { get; set; } = 0;

    public List<string> ExcludedApps { get; set; } = new()
    {
        "valorant.exe",
        "cs2.exe",
        "League of Legends.exe",
        "Overwatch.exe",
        "dota2.exe"
    };

    public Dictionary<string, string> Macros { get; set; } = new(StringComparer.OrdinalIgnoreCase)
    {
        ["vf"] = "VietFlow",
        ["vn"] = "Việt Nam",
        ["chxhcnvn"] = "Cộng hòa Xã hội Chủ nghĩa Việt Nam",
        ["dt"] = "Hôm nay là @date, bây giờ là @time"
    };

    public static string GetConfigDirectory()
    {
        string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        string dir = Path.Combine(appData, "VietFlow");
        if (!Directory.Exists(dir))
        {
            Directory.CreateDirectory(dir);
        }
        return dir;
    }

    public static string GetConfigFilePath() =>
        Path.Combine(GetConfigDirectory(), "vietflow_config.json");

    public static VietFlowConfig Load()
    {
        string path = GetConfigFilePath();
        try
        {
            if (File.Exists(path))
            {
                string json = File.ReadAllText(path);
                var config = JsonSerializer.Deserialize(json, VietFlowJsonContext.Default.VietFlowConfig);
                if (config != null)
                {
                    return config;
                }
            }
        }
        catch
        {
            // Fallback to default on corrupted config
        }

        var def = new VietFlowConfig();
        def.Save();
        return def;
    }

    public void Save()
    {
        try
        {
            string path = GetConfigFilePath();
            string json = JsonSerializer.Serialize(this, VietFlowJsonContext.Default.VietFlowConfig);
            File.WriteAllText(path, json);
        }
        catch
        {
            // Ignore I/O errors during save
        }
    }

    public void ApplyToSession(VietFlowSession session)
    {
        session.IsEnabled = IsEnabled;
        session.Options.Method = Method;
        session.Options.ModernStyle = ModernStyle;
        session.Options.FreeMarking = FreeMarking;
        session.Options.EnableMacro = EnableMacro;
        session.Options.SpellCheck = SpellCheck;
        session.Options.AutoNonVnRestore = AutoNonVnRestore;
        session.Options.FixShiftLag = FixShiftLag;
        session.Options.AutoCapitalize = AutoCapitalize;
        session.Options.ForeignInitials = string.IsNullOrWhiteSpace(ForeignInitials) ? "fjwz" : ForeignInitials;
        session.Options.ConsonantShortcuts = ConsonantShortcuts;
        session.Charset = Charsets.CharsetFactory.GetCharset(Charset);

        EnglishPhonetics.SetKeywords(EnglishKeywords.Count > 0 ? EnglishKeywords : EnglishPhonetics.DefaultKeywords);

        session.Macros.Clear();
        foreach (var (k, v) in Macros)
        {
            session.Macros.AddOrUpdate(k, v);
        }
    }
}

