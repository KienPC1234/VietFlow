#include "config.h"
#include "engine.h"
#include "vietflow_core.h"

#include <cassert>
#include <dlfcn.h>
#include <fcitx-config/rawconfig.h>
#include <iostream>
#include <string>

namespace {

std::string takeUtf8(VfOwnedUtf8 &owned) {
    std::string out;
    if (owned.data != nullptr && owned.len > 0) {
        out.assign(reinterpret_cast<const char *>(owned.data), owned.len);
    }
    vf_owned_utf8_free(owned);
    owned.data = nullptr;
    owned.len = 0;
    return out;
}

std::string typeWordTelex(VfEngine *engine, const std::string &keys) {
    std::string preedit;
    for (char c : keys) {
        VfOwnedUtf8 out = {};
        const uint8_t action = vf_engine_process_ime_key_owned(
            engine, static_cast<uint32_t>(static_cast<unsigned char>(c)), 0,
            &out);
        preedit = takeUtf8(out);
        assert(action == VF_IME_UPDATE_PREEDIT);
    }
    return preedit;
}

void testConfigRoundtripAndSync() {
    vietflow::VietFlowConfig cfg;
    fcitx::RawConfig raw;
    raw.setValueByPath("InputMethod", "VNI");
    raw.setValueByPath("Charset", "Unicode (NFC)");
    raw.setValueByPath("SpellCheck", "True");
    raw.setValueByPath("ModernStyle", "True");
    raw.setValueByPath("AutoRestore", "True");
    raw.setValueByPath("EnableMacro", "True");
    raw.setValueByPath("ForeignInitials", "zw");
    raw.setValueByPath("CustomEnglishWords/0", "vietflow");
    raw.setValueByPath("RemovedEnglishWords/0", "test");
    raw.setValueByPath("Macros/0/Trigger", "vf");
    raw.setValueByPath("Macros/0/Replacement", "VietFlow");
    raw.setValueByPath("ExcludedApps/0", "code");

    cfg.load(raw, true);
    assert(cfg.excludedApps->size() == 1);
    assert(cfg.excludedApps->at(0) == "code");
    assert(*cfg.inputMethod == vietflow::VfMethodOption::Vni);
    assert(*cfg.foreignInitials == "zw");
    assert(cfg.customEnglishWords->size() == 1);
    assert(cfg.customEnglishWords->at(0) == "vietflow");
    assert(cfg.macros->size() == 1);
    assert(*cfg.macros->at(0).trigger == "vf");
    assert(*cfg.macros->at(0).replacement == "VietFlow");

    vietflow::VietFlowState state;
    state.syncConfig(cfg);

    // VNI: v-i-e-6-t-5 -> "việt"
    const std::string vniPreedit = typeWordTelex(state.engine(), "vie6t5");
    assert(vniPreedit == "việt");

    VfOwnedUtf8 fin = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &fin));
    assert(takeUtf8(fin) == "việt");
    vf_engine_reset(state.engine());

    // Macro: "vf" + break -> "VietFlow"
    assert(typeWordTelex(state.engine(), "vf") == "vf");
    VfOwnedUtf8 macroOut = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &macroOut));
    assert(takeUtf8(macroOut) == "VietFlow");
    vf_engine_reset(state.engine());

    // Custom English word "vietflow" should restore to "vietflow" on break
    typeWordTelex(state.engine(), "vietflow");
    VfOwnedUtf8 kwOut = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &kwOut));
    assert(takeUtf8(kwOut) == "vietflow");
}

void testTelexBackspaceAndAutoRestore() {
    vietflow::VietFlowConfig cfg;
    vietflow::VietFlowState state;
    state.syncConfig(cfg);

    assert(typeWordTelex(state.engine(), "vieetj") == "việt");

    VfOwnedUtf8 bs = {};
    assert(vf_engine_process_backspace_owned(state.engine(), &bs));
    assert(takeUtf8(bs) == "việ");

    // Finish remaining word
    VfOwnedUtf8 fin = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &fin));
    assert(takeUtf8(fin) == "việ");
    vf_engine_reset(state.engine());

    // English word auto-restore: "text" -> "têxt" during typing, restored to "text"
    typeWordTelex(state.engine(), "text");
    VfOwnedUtf8 enOut = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &enOut));
    assert(takeUtf8(enOut) == "text");
}

void testBidirectionalAutoSwitch() {
    vietflow::VietFlowConfig cfg;
    assert(*cfg.autoSwitchByTyping == true);
    assert(*cfg.freeMarking == true);
    assert(*cfg.fixShiftLag == true);

    vietflow::VietFlowState state;
    state.syncConfig(cfg);

    // Typing an English word that gets mangled in Telex ("world" -> "ơrld" / "wỏld")
    // should restore "world" on break and request switch to English (0).
    typeWordTelex(state.engine(), "world");
    VfOwnedUtf8 enFin = {};
    assert(vf_engine_finish_on_break_owned(state.engine(), ' ', &enFin));
    assert(takeUtf8(enFin) == "world");
    assert(vf_engine_take_auto_switch_request(state.engine()) == 0);

    // Push another English word to lookback while in English mode.
    const std::string enWord = "hello";
    vf_engine_push_english_word_lookback(
        state.engine(), reinterpret_cast<const uint8_t *>(enWord.data()),
        enWord.size());

    // Now type a Vietnamese word in English mode ("vieetj") -> should detect VN,
    // return replacement "việt " with 6 backspaces, and request switch to VN (1).
    const std::string vnRaw = "vieetj";
    VfOwnedUtf8 vnOut = {};
    size_t backspaces = 0;
    assert(vf_engine_check_english_word_at_break_owned(
        state.engine(), reinterpret_cast<const uint8_t *>(vnRaw.data()),
        vnRaw.size(), ' ', &vnOut, &backspaces));
    assert(backspaces == 6);
    assert(takeUtf8(vnOut) == "việt ");
    assert(vf_engine_take_auto_switch_request(state.engine()) == 1);
}

void testSharedModuleDlopen() {
    const std::string soPath = VIETFLOW_ADDON_SO_PATH;
    void *handle = dlopen(soPath.c_str(), RTLD_NOW);
    if (handle == nullptr) {
        std::cerr << "dlopen failed: " << dlerror() << "\n";
        assert(false);
    }
    void *sym = dlsym(handle, "fcitx_addon_factory_instance_vietflow");
    if (sym == nullptr) {
        sym = dlsym(handle, "fcitx_addon_factory_instance");
    }
    assert(sym != nullptr);
    dlclose(handle);
}

} // namespace

int main() {
    testConfigRoundtripAndSync();
    testTelexBackspaceAndAutoRestore();
    testBidirectionalAutoSwitch();
    testSharedModuleDlopen();
    std::cout << "All Fcitx5 addon tests passed.\n";
    return 0;
}
