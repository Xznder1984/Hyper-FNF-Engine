// Canonical launcher UI source (Haxe, target JS).
//
// The checked-in frontend/app.js is the hand-maintained JS fallback and should
// stay in sync with the declarations below. Regenerate / verify with:
//
//   haxe build.hxml        (outputs ../frontend/app.js)
//
// Haxe is not required to build the launcher; it is the source of record for
// the UI bridge types and app logic so maintainers can regenerate the JS.
import js.html.Dom;
import js.lib.Promise;

typedef ModDetect = {
	engine_id: Null<String>,
	confidence: String,
	candidates: Array<{ engine_id:String, score:Int, evidence:Array<String> }>,
	evidence: Array<String>,
}

typedef EngineInfo = {
	id:String,
	name:String,
	license:Null<String>,
	homepage:Null<String>,
	repo:Null<String>,
	notes:Array<String>,
	installed:Array<String>,
	sarahud:Null<String>,
}

typedef LaunchOutcome = {
	pid:Int,
	exe:String,
}

class Main {
	static function main() {
		// Bootstrap lives in frontend/app.js (the compiled JS fallback).
		// This class is the reference implementation of the same logic in
		// typed Haxe; see README.md in this crate for the sync contract.
		final win = js.Browser.window;
		trace("Hyper Engine UI loaded", win.location.href);
	}
}