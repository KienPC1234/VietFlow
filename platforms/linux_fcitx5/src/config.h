#ifndef VIETFLOW_FCITX5_CONFIG_H
#define VIETFLOW_FCITX5_CONFIG_H

#include <fcitx-config/configuration.h>
#include <fcitx-config/enum.h>
#include <fcitx-utils/i18n.h>
#include <fcitx-utils/key.h>
#include <string>
#include <vector>

namespace vietflow {

enum class VfMethodOption {
    Telex = 0,
    Vni = 1,
    Viqr = 2,
    SimpleTelex = 3,
    SimpleTelex2 = 4,
    TelexVni = 5,
};

FCITX_CONFIG_ENUM_NAME_WITH_I18N(VfMethodOption,
                                N_("Telex"),
                                N_("VNI"),
                                N_("VIQR"),
                                N_("Simple Telex"),
                                N_("Simple Telex 2"),
                                N_("Telex + VNI"));

enum class VfCharsetOption {
    Unicode = 0,
    UnicodeComposite = 1,
    Tcvn3 = 2,
    VniWindows = 3,
    Viqr = 4,
};

FCITX_CONFIG_ENUM_NAME_WITH_I18N(VfCharsetOption,
                                N_("Unicode (NFC)"),
                                N_("Unicode Composite (NFD)"),
                                N_("TCVN3 (ABC)"),
                                N_("VNI Windows"),
                                N_("VIQR"));

FCITX_CONFIGURATION(
    MacroEntryConfig,
    fcitx::Option<std::string> trigger{this, "Trigger", _("Trigger"), ""};
    fcitx::Option<std::string> replacement{this, "Replacement",
                                           _("Replacement"), ""};
);

FCITX_CONFIGURATION(
    VietFlowConfig,
    fcitx::OptionWithAnnotation<VfMethodOption, VfMethodOptionI18NAnnotation>
        inputMethod{this, "InputMethod", _("Input Method"),
                    VfMethodOption::Telex};
    fcitx::OptionWithAnnotation<VfCharsetOption, VfCharsetOptionI18NAnnotation>
        charset{this, "Charset", _("Output Charset"),
                VfCharsetOption::Unicode};
    fcitx::Option<bool> spellCheck{this, "SpellCheck", _("Spell Check"), true};
    fcitx::Option<bool> modernStyle{this, "ModernStyle",
                                    _("Modern Tone Placement (oà, uý)"), true};
    fcitx::Option<bool> autoRestore{
        this, "AutoRestore", _("Auto-Restore English Words"), true};
    fcitx::Option<bool> freeMarking{
        this, "FreeMarking", _("Free Tone Marking"), true};
    fcitx::Option<bool> enableMacro{this, "EnableMacro", _("Enable Macros"),
                                    true};
    fcitx::Option<bool> autoSwitchByTyping{
        this, "AutoSwitchByTyping", _("Auto-Switch by Typing"), true};
    fcitx::Option<bool> fixShiftLag{this, "FixShiftLag", _("Fix Shift Lag"),
                                    true};
    fcitx::Option<fcitx::KeyList> toggleModeKey{
        this, "ToggleModeKey", _("Toggle Vietnamese / English"), {}};
    fcitx::Option<std::vector<std::string>> excludedApps{
        this,
        "ExcludedApps",
        _("Disabled Apps (e.g. code, kitty, alacritty)"),
        {}};
    fcitx::Option<std::string> foreignInitials{
        this, "ForeignInitials", _("Foreign Initials (e.g. zwjf)"), ""};
    fcitx::Option<std::vector<std::string>> customEnglishWords{
        this, "CustomEnglishWords", _("Custom English Words"), {}};
    fcitx::Option<std::vector<std::string>> removedEnglishWords{
        this, "RemovedEnglishWords", _("Removed English Words"), {}};
    fcitx::Option<std::vector<MacroEntryConfig>> macros{
        this, "Macros", _("Macros"), {}};
);

} // namespace vietflow

#endif // VIETFLOW_FCITX5_CONFIG_H
