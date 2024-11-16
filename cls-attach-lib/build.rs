fn main() {
    println!("cargo:rustc-link-search=native=/Users/anatawa12/RustroverProjects/console-log-saver/cls-attach-lib/mono");
}
/*
mkdir -p mono
cp /Applications/Unity/Hub/Editor/2022.3.22f1/Unity.app/Contents/Frameworks/MonoBleedingEdge/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib mono/libmonobdwgc-2.0.dylib
install_name_tool -change @executable_path/../Frameworks/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib @executable_path/../Frameworks/MonoBleedingEdge/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib mono/libmonobdwgc-2.0.dylib

install_name_tool -change @executable_path/../Frameworks/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib @executable_path/../Frameworks/MonoBleedingEdge/MonoEmbedRuntime/osx/libmonobdwgc-2.0.dylib target/debug/libcls_attach_lib.dylib
 */
