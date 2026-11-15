// Tauri v2 IPC shim: lets the unchanged desktop React UI run in an Android WebView.
// `@tauri-apps/api` only calls window.__TAURI_INTERNALS__, so commands go to the Kotlin
// bridge (`DialBridge`, see web/WebBridge.kt) and Kotlin pushes events back via __DC_EMIT__.
(function () {
  window.__DIAL_PLATFORM__ = "android";
  var callbacks = {};
  var nextCallback = 1;
  var pending = {};
  var nextRequest = 1;
  var listeners = {}; // event -> [{ id, handler }]
  var nextEventId = 1;

  function transformCallback(cb, once) {
    var id = nextCallback++;
    callbacks[id] = function (payload) {
      if (once) delete callbacks[id];
      if (cb) cb(payload);
    };
    return id;
  }

  function listen(args) {
    var id = nextEventId++;
    (listeners[args.event] = listeners[args.event] || []).push({ id: id, handler: args.handler });
    return Promise.resolve(id);
  }

  function unlisten(event, eventId) {
    listeners[event] = (listeners[event] || []).filter(function (l) { return l.id !== eventId; });
  }

  window.__DC_EMIT__ = function (event, payload) {
    (listeners[event] || []).slice().forEach(function (l) {
      var cb = callbacks[l.handler];
      if (cb) cb({ event: event, id: l.id, payload: payload });
    });
  };

  window.__DC_RESOLVE__ = function (requestId, ok, value) {
    var p = pending[requestId];
    if (!p) return;
    delete pending[requestId];
    (ok ? p.resolve : p.reject)(value);
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: unlisten };

  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "android" }, currentWebview: { label: "android" } },
    transformCallback: transformCallback,
    unregisterCallback: function (id) { delete callbacks[id]; },
    convertFileSrc: function (path) { return path; },
    invoke: function (cmd, args) {
      args = args || {};
      if (cmd === "plugin:event|listen") return listen(args);
      if (cmd === "plugin:event|unlisten") { unlisten(args.event, args.eventId); return Promise.resolve(); }
      if (cmd === "plugin:event|emit") { window.__DC_EMIT__(args.event, args.payload); return Promise.resolve(); }
      return new Promise(function (resolve, reject) {
        var id = nextRequest++;
        pending[id] = { resolve: resolve, reject: reject };
        window.DialBridge.invoke(cmd, JSON.stringify(args), id);
      });
    },
  };
})();
