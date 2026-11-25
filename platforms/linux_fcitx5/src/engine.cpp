#include "engine.h"

#include <algorithm>
#include <array>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>

namespace vietflow {

namespace {

constexpr const char *kConfigPath = "conf/vietflow.conf";
// Matches crate::macro_table::CLIPBOARD_PLACEHOLDER ('\u{E000}').
constexpr const char *kClipboardPlaceholderUtf8 = "\xEE\x80\x80";

uint64_t nowMs() {
    using namespace std::chrono;
    return static_cast<uint64_t>(
        duration_cast<milliseconds>(steady_clock::now().time_since_epoch())
            .count());
}

std::string takeOwnedUtf8(VfOwnedUtf8 &owned) {
    std::string out;
    if (owned.data != nullptr && owned.len > 0) {
        out.assign(reinterpret_cast<const char *>(owned.data), owned.len);
    }
    vf_owned_utf8_free(owned);
    owned.data = nullptr;
    owned.len = 0;
    return out;
}

std::string toLowerAscii(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(), [](unsigned char c) {
        return static_cast<char>(std::tolower(c));
    });
    return s;
}

std::string basenameLower(const std::string &path) {
    const size_t pos = path.find_last_of('/');
    return toLowerAscii(pos == std::string::npos ? path : path.substr(pos + 1));
}
std::string joinNewline(const std::vector<std::string> &items) {
    std::string out;
    for (const auto &item : items) {
        if (!item.empty()) {
            out.append(item);
            out.push_back('\n');
        }
    }
    return out;
}

std::string readCommandOutput(const char *cmd) {
    FILE *pipe = popen(cmd, "r");
    if (pipe == nullptr) {
        return {};
    }
    std::string result;
    std::array<char, 512> buf{};
    while (std::fgets(buf.data(), static_cast<int>(buf.size()), pipe) !=
           nullptr) {
        result.append(buf.data());
    }
    if (pclose(pipe) != 0) {
        return {};
    }
    return result;
}

std::string readSystemClipboard() {
    if (std::getenv("WAYLAND_DISPLAY") != nullptr) {
        std::string out = readCommandOutput("wl-paste -n 2>/dev/null");
        if (!out.empty()) {
            return out;
        }
    }
    if (std::getenv("DISPLAY") != nullptr) {
        std::string out =
            readCommandOutput("xclip -o -selection clipboard 2>/dev/null");
        if (!out.empty()) {
            return out;
        }
        return readCommandOutput("xsel -ob 2>/dev/null");
    }
    return {};
}

void expandClipboardPlaceholder(std::string &text) {
    if (text.find(kClipboardPlaceholderUtf8) == std::string::npos) {
        return;
    }
    const std::string clip = readSystemClipboard();
    const std::string placeholder(kClipboardPlaceholderUtf8);
    size_t pos = 0;
    while ((pos = text.find(placeholder, pos)) != std::string::npos) {
        text.replace(pos, placeholder.size(), clip);
        pos += clip.size();
    }
}

bool isNavigationKey(fcitx::KeySym sym) {
    switch (sym) {
    case FcitxKey_Left:
    case FcitxKey_Up:
    case FcitxKey_Right:
    case FcitxKey_Down:
    case FcitxKey_Home:
    case FcitxKey_End:
    case FcitxKey_Page_Up:
    case FcitxKey_Page_Down:
    case FcitxKey_Delete:
        return true;
    default:
        return false;
    }
}

uint32_t keySymToChar(fcitx::KeySym sym) {
    if (const uint32_t ch = fcitx::Key::keySymToUnicode(sym); ch != 0) {
        return ch;
    }
    if (sym >= FcitxKey_KP_0 && sym <= FcitxKey_KP_9) {
        return static_cast<uint32_t>('0' + (sym - FcitxKey_KP_0));
    }
    switch (sym) {
    case FcitxKey_KP_Decimal:
        return '.';
    case FcitxKey_KP_Divide:
        return '/';
    case FcitxKey_KP_Multiply:
        return '*';
    case FcitxKey_KP_Subtract:
        return '-';
    case FcitxKey_KP_Add:
        return '+';
    default:
        return 0;
    }
}

} // namespace

VietFlowState::VietFlowState() : engine_(vf_engine_create()) {}

VietFlowState::~VietFlowState() {
    if (engine_ != nullptr) {
        vf_engine_destroy(engine_);
        engine_ = nullptr;
    }
}

void VietFlowState::syncConfig(const VietFlowConfig &config) {
    if (engine_ == nullptr) {
        return;
    }
    vf_engine_set_method(engine_, static_cast<uint8_t>(*config.inputMethod));
    vf_engine_set_charset(engine_, static_cast<uint8_t>(*config.charset));
    vf_engine_set_spell_check(engine_, *config.spellCheck);
    vf_engine_set_modern_style(engine_, *config.modernStyle);
    vf_engine_set_auto_restore(engine_, *config.autoRestore);
    vf_engine_set_free_marking(engine_, *config.freeMarking);
    vf_engine_set_enable_macro(engine_, *config.enableMacro);
    vf_engine_set_auto_switch_by_typing(engine_, *config.autoSwitchByTyping);
    vf_engine_set_fix_shift_lag(engine_, *config.fixShiftLag);
    vf_engine_set_typing_timeout_ms(engine_, 0);

    const std::string &foreign = *config.foreignInitials;
    vf_engine_set_foreign_initials(
        engine_, reinterpret_cast<const uint8_t *>(foreign.data()),
        foreign.size());

    const std::string customKw = joinNewline(*config.customEnglishWords);
    vf_engine_set_custom_english_keywords(
        engine_, reinterpret_cast<const uint8_t *>(customKw.data()),
        customKw.size());

    const std::string removedKw = joinNewline(*config.removedEnglishWords);
    vf_engine_set_removed_english_keywords(
        engine_, reinterpret_cast<const uint8_t *>(removedKw.data()),
        removedKw.size());

    vf_engine_clear_macros(engine_);
    for (const auto &entry : *config.macros) {
        const std::string &trig = *entry.trigger;
        const std::string &repl = *entry.replacement;
        if (!trig.empty()) {
            vf_engine_add_macro(
                engine_, reinterpret_cast<const uint8_t *>(trig.data()),
                trig.size(), reinterpret_cast<const uint8_t *>(repl.data()),
                repl.size());
        }
    }
}

void VietFlowState::resetEngine() {
    if (engine_ != nullptr) {
        vf_engine_reset(engine_);
    }
}

VietFlowEngine::VietFlowEngine(fcitx::Instance *instance)
    : instance_(instance), factory_([this](fcitx::InputContext &) {
          auto *state = new VietFlowState();
          state->syncConfig(config_);
          return state;
      }) {
    instance_->inputContextManager().registerProperty("vietflowState",
                                                      &factory_);
    reloadConfig();
}

void VietFlowEngine::reloadConfig() {
    fcitx::readAsIni(config_, kConfigPath);
    instance_->inputContextManager().foreach([this](fcitx::InputContext *ic) {
        if (auto *state = ic->propertyFor(&factory_)) {
            state->syncConfig(config_);
        }
        return true;
    });
}

void VietFlowEngine::setConfig(const fcitx::RawConfig &rawConfig) {
    config_.load(rawConfig, true);
    fcitx::safeSaveAsIni(config_, kConfigPath);
    reloadConfig();
}

std::string VietFlowEngine::subModeLabelImpl(const fcitx::InputMethodEntry &,
                                             fcitx::InputContext &ic) {
    if (isAppExcluded(&ic)) {
        return "EN";
    }
    if (const auto *state = ic.propertyFor(&factory_)) {
        if (!state->vietnameseMode()) {
            return "EN";
        }
    }
    switch (*config_.inputMethod) {
    case VfMethodOption::Telex:
        return "Telex";
    case VfMethodOption::Vni:
        return "VNI";
    case VfMethodOption::Viqr:
        return "VIQR";
    case VfMethodOption::SimpleTelex:
        return "STelex";
    case VfMethodOption::SimpleTelex2:
        return "STelex2";
    case VfMethodOption::TelexVni:
        return "T+VNI";
    }
    return "VI";
}

void VietFlowEngine::clearPreedit(fcitx::InputContext *ic,
                                  VietFlowState *state) {
    ic->inputPanel().reset();
    ic->updatePreedit();
    ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    state->resetEngine();
}

void VietFlowEngine::updatePreeditText(fcitx::InputContext *ic,
                                       const std::string &utf8) {
    const bool useClientPreedit =
        ic->capabilityFlags().test(fcitx::CapabilityFlag::Preedit) &&
        ic->isPreeditEnabled();

    fcitx::Text preedit;
    if (!utf8.empty()) {
        const auto formatFlag =
            (useClientPreedit &&
             ic->capabilityFlags().test(fcitx::CapabilityFlag::FormattedPreedit))
                ? fcitx::TextFormatFlag::Underline
                : fcitx::TextFormatFlag::NoFlag;
        preedit.append(utf8, formatFlag);
        preedit.setCursor(static_cast<int>(utf8.size()));
    }

    if (useClientPreedit) {
        ic->inputPanel().setClientPreedit(preedit);
        ic->updatePreedit();
    } else {
        ic->inputPanel().setPreedit(preedit);
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }
}

void VietFlowEngine::commitCurrentPreedit(fcitx::InputContext *ic,
                                          VietFlowState *state) {
    VfOwnedUtf8 output = {};
    std::string text;
    if (vf_engine_get_preedit_owned(state->engine(), &output)) {
        text = takeOwnedUtf8(output);
    }
    clearPreedit(ic, state);
    if (!text.empty()) {
        ic->commitString(text);
    }
}

void VietFlowEngine::commitWithBreak(fcitx::InputContext *ic,
                                     VietFlowState *state,
                                     uint32_t breakChar) {
    VfOwnedUtf8 output = {};
    if (vf_engine_finish_on_break_owned(
            state->engine(), breakChar != 0 ? breakChar : ' ', &output)) {
        std::string text = takeOwnedUtf8(output);
        expandClipboardPlaceholder(text);
        if (breakChar != 0 && breakChar < 0x80) {
            text.push_back(static_cast<char>(breakChar));
        }
        clearPreedit(ic, state);
        if (!text.empty()) {
            ic->commitString(text);
        }
        applyAutoSwitchRequest(ic, state);
    } else {
        clearPreedit(ic, state);
    }
}

bool VietFlowEngine::isAppExcluded(const fcitx::InputContext *ic) const {
    if (ic == nullptr || config_.excludedApps->empty()) {
        return false;
    }
    const std::string &prog = ic->program();
    if (prog.empty()) {
        return false;
    }
    const std::string progBase = basenameLower(prog);
    const std::string progFull = toLowerAscii(prog);
    for (const auto &rawPattern : *config_.excludedApps) {
        const std::string pattern = toLowerAscii(rawPattern);
        if (pattern.empty()) {
            continue;
        }
        if (progBase == pattern || progFull == pattern ||
            progBase.find(pattern) != std::string::npos) {
            return true;
        }
    }
    return false;
}

void VietFlowEngine::applyAutoSwitchRequest(fcitx::InputContext *ic,
                                            VietFlowState *state) {
    const int8_t req = vf_engine_take_auto_switch_request(state->engine());
    if (req == 0 && state->vietnameseMode()) {
        // Only enter raw English passthrough when the client supports
        // SurroundingText so [E] -> [V] can rewrite the transition word.
        if (ic->capabilityFlags().test(
                fcitx::CapabilityFlag::SurroundingText)) {
            state->setVietnameseMode(false);
            state->clearEnBuffer();
            ic->updateUserInterface(fcitx::UserInterfaceComponent::StatusArea);
        }
    } else if (req == 1 && !state->vietnameseMode()) {
        state->setVietnameseMode(true);
        state->clearEnBuffer();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::StatusArea);
    }
}

void VietFlowEngine::handleEnglishModeBreak(fcitx::InputContext *ic,
                                            VietFlowState *state,
                                            uint32_t breakChar,
                                            fcitx::KeyEvent &event,
                                            bool consumeOnCommit) {
    if (state->enBuffer().empty()) {
        return;
    }

    const std::string rawWord = state->enBuffer();
    state->clearEnBuffer();

    VfOwnedUtf8 output = {};
    size_t backspaces = 0;
    if (vf_engine_check_english_word_at_break_owned(
            state->engine(), reinterpret_cast<const uint8_t *>(rawWord.data()),
            rawWord.size(), breakChar, &output, &backspaces)) {
        const std::string composed = takeOwnedUtf8(output);
        applyAutoSwitchRequest(ic, state);
        if (!composed.empty() &&
            ic->capabilityFlags().test(
                fcitx::CapabilityFlag::SurroundingText) &&
            backspaces > 0) {
            ic->deleteSurroundingText(-static_cast<int>(backspaces),
                                      static_cast<unsigned int>(backspaces));
            ic->commitString(composed);
            if (consumeOnCommit) {
                event.filterAndAccept();
            }
        }
        return;
    }

    vf_engine_push_english_word_lookback(
        state->engine(), reinterpret_cast<const uint8_t *>(rawWord.data()),
        rawWord.size());
}

void VietFlowEngine::handleEnglishModeKey(fcitx::InputContext *ic,
                                          VietFlowState *state,
                                          fcitx::KeyEvent &event) {
    const fcitx::Key key = event.key();
    const fcitx::KeyStates nonShiftMods{
        fcitx::KeyState::Ctrl, fcitx::KeyState::Alt, fcitx::KeyState::Super,
        fcitx::KeyState::Hyper};
    if (key.states().testAny(nonShiftMods)) {
        state->clearEnBuffer();
        return;
    }

    const fcitx::KeySym sym = key.sym();
    if (sym == FcitxKey_BackSpace) {
        if (!state->enBuffer().empty()) {
            state->enBuffer().pop_back();
        }
        return;
    }

    if (sym == FcitxKey_Escape || isNavigationKey(sym)) {
        state->clearEnBuffer();
        return;
    }

    if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter ||
        sym == FcitxKey_Tab) {
        handleEnglishModeBreak(ic, state, 0, event, false);
        return;
    }

    if (sym == FcitxKey_space) {
        handleEnglishModeBreak(ic, state, ' ', event, true);
        return;
    }

    const uint32_t ch = keySymToChar(sym);
    if (ch == 0) {
        state->clearEnBuffer();
        return;
    }

    const bool isWordChar =
        (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') ||
        (ch >= '0' && ch <= '9') ||
        (*config_.inputMethod == VfMethodOption::Telex &&
         (ch == '[' || ch == ']' || ch == '{' || ch == '}'));

    if (isWordChar) {
        if (state->enBuffer().size() < 32) {
            state->enBuffer().push_back(static_cast<char>(ch));
        } else {
            state->clearEnBuffer();
        }
        return;
    }

    handleEnglishModeBreak(ic, state, ch, event, true);
}

void VietFlowEngine::handleBackspace(fcitx::InputContext *ic,
                                     VietFlowState *state,
                                     fcitx::KeyEvent &event) {
    if (vf_engine_is_empty(state->engine())) {
        return;
    }

    VfOwnedUtf8 output = {};
    if (!vf_engine_process_backspace_owned(state->engine(), &output)) {
        clearPreedit(ic, state);
        return;
    }

    std::string remaining = takeOwnedUtf8(output);
    if (remaining.empty()) {
        clearPreedit(ic, state);
    } else {
        updatePreeditText(ic, remaining);
    }
    event.filterAndAccept();
}

void VietFlowEngine::keyEvent(const fcitx::InputMethodEntry &,
                              fcitx::KeyEvent &event) {
    if (event.isRelease()) {
        return;
    }

    auto *ic = event.inputContext();
    auto *state = ic->propertyFor(&factory_);
    if (state == nullptr || state->engine() == nullptr) {
        return;
    }

    if (ic->capabilityFlags().test(fcitx::CapabilityFlag::Password) ||
        isAppExcluded(ic)) {
        if (!vf_engine_is_empty(state->engine())) {
            clearPreedit(ic, state);
        }
        state->clearEnBuffer();
        return;
    }

    const fcitx::Key key = event.key();
    if (!config_.toggleModeKey->empty() &&
        key.checkKeyList(*config_.toggleModeKey)) {
        if (!vf_engine_is_empty(state->engine())) {
            commitCurrentPreedit(ic, state);
        }
        state->clearEnBuffer();
        state->setVietnameseMode(!state->vietnameseMode());
        ic->updateUserInterface(fcitx::UserInterfaceComponent::StatusArea);
        event.filterAndAccept();
        return;
    }

    if (key.isModifier()) {
        return;
    }

    if (!state->vietnameseMode()) {
        handleEnglishModeKey(ic, state, event);
        return;
    }

    const fcitx::KeyStates nonShiftMods{
        fcitx::KeyState::Ctrl, fcitx::KeyState::Alt, fcitx::KeyState::Super,
        fcitx::KeyState::Hyper};
    if (key.states().testAny(nonShiftMods)) {
        if (!vf_engine_is_empty(state->engine())) {
            commitWithBreak(ic, state, 0);
        }
        return;
    }

    const fcitx::KeySym sym = key.sym();
    if (sym == FcitxKey_BackSpace) {
        handleBackspace(ic, state, event);
        return;
    }

    if (sym == FcitxKey_Escape) {
        if (!vf_engine_is_empty(state->engine())) {
            commitCurrentPreedit(ic, state);
            event.filterAndAccept();
        }
        return;
    }

    if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter ||
        sym == FcitxKey_Tab) {
        if (!vf_engine_is_empty(state->engine())) {
            commitWithBreak(ic, state, 0);
        }
        return;
    }

    if (sym == FcitxKey_space) {
        if (!vf_engine_is_empty(state->engine())) {
            commitWithBreak(ic, state, ' ');
            event.filterAndAccept();
        }
        return;
    }

    if (isNavigationKey(sym)) {
        if (!vf_engine_is_empty(state->engine())) {
            commitWithBreak(ic, state, 0);
        }
        return;
    }

    const uint32_t ch = keySymToChar(sym);
    if (ch == 0) {
        if (!vf_engine_is_empty(state->engine())) {
            commitWithBreak(ic, state, 0);
        }
        return;
    }

    VfOwnedUtf8 output = {};
    const uint8_t action =
        vf_engine_process_ime_key_owned(state->engine(), ch, nowMs(), &output);
    const std::string text = takeOwnedUtf8(output);

    switch (action) {
    case VF_IME_PASSTHROUGH:
        return;
    case VF_IME_UPDATE_PREEDIT:
        if (text.empty()) {
            clearPreedit(ic, state);
        } else {
            updatePreeditText(ic, text);
        }
        event.filterAndAccept();
        return;
    case VF_IME_COMMIT_CONSUME: {
        std::string committed = text;
        expandClipboardPlaceholder(committed);
        clearPreedit(ic, state);
        if (!committed.empty()) {
            ic->commitString(committed);
        }
        applyAutoSwitchRequest(ic, state);
        event.filterAndAccept();
        return;
    }
    case VF_IME_COMMIT_FORWARD: {
        std::string committed = text;
        expandClipboardPlaceholder(committed);
        clearPreedit(ic, state);
        if (!committed.empty()) {
            ic->commitString(committed);
        }
        applyAutoSwitchRequest(ic, state);
        return;
    }
    default:
        return;
    }
}

void VietFlowEngine::activate(const fcitx::InputMethodEntry &,
                              fcitx::InputContextEvent &event) {
    auto *ic = event.inputContext();
    if (auto *state = ic->propertyFor(&factory_)) {
        state->syncConfig(config_);
    }
}

void VietFlowEngine::deactivate(const fcitx::InputMethodEntry &,
                                fcitx::InputContextEvent &event) {
    auto *ic = event.inputContext();
    if (auto *state = ic->propertyFor(&factory_)) {
        state->clearEnBuffer();
        if (!vf_engine_is_empty(state->engine())) {
            commitCurrentPreedit(ic, state);
        } else {
            clearPreedit(ic, state);
        }
    }
}

void VietFlowEngine::reset(const fcitx::InputMethodEntry &,
                           fcitx::InputContextEvent &event) {
    auto *ic = event.inputContext();
    if (auto *state = ic->propertyFor(&factory_)) {
        state->clearEnBuffer();
        clearPreedit(ic, state);
    }
}

} // namespace vietflow

#ifdef FCITX_ADDON_FACTORY_V2
FCITX_ADDON_FACTORY_V2(vietflow, vietflow::VietFlowFactory)
#else
FCITX_ADDON_FACTORY(vietflow::VietFlowFactory)
#endif
