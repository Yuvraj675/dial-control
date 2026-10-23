import { Extension, gettext as _ } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';

const DBUS_SCHEMA = `
<node>
    <interface name="org.gnome.shell.extensions.FocusedWindow">
        <method name="Get">
            <arg type="s" direction="out" name="window" />
        </method>
        <method name="MoveWindow">
            <arg type="s" direction="in" name="title" />
            <arg type="i" direction="in" name="x" />
            <arg type="i" direction="in" name="y" />
        </method>
        <signal name="FocusChanged">
            <arg type="s" name="window" />
        </signal>
    </interface>
</node>`;

function hasValidMonitor(metaWindow) {
  const windowMonitor = metaWindow.get_monitor();
  if (windowMonitor < 0) {
    return false;
  }
  const numMonitors = global.display.get_n_monitors();
  if (windowMonitor >= numMonitors) {
    return false;
  }
  return true;
}

export default class FocusedWindowDbus extends Extension {
  Get() {
    let window_list = global.get_window_actors();
    let focusedWindow = window_list.find((window) =>
      window.meta_window.has_focus()
    );

    let workspaceManager = global.workspace_manager;
    let currentmonitor = global.display.get_current_monitor();
    if (focusedWindow) {
      const validMonitor = hasValidMonitor(focusedWindow.meta_window);

      return JSON.stringify({
        title: focusedWindow.meta_window.get_title(),
        wm_class: focusedWindow.meta_window.get_wm_class(),
        wm_class_instance: focusedWindow.meta_window.get_wm_class_instance(),
        pid: focusedWindow.meta_window.get_pid(),
        id: focusedWindow.meta_window.get_id(),
        width: focusedWindow.get_width(),
        height: focusedWindow.get_height(),
        x: focusedWindow.get_x(),
        y: focusedWindow.get_y(),
        focus: focusedWindow.meta_window.has_focus(),
        in_current_workspace: focusedWindow.meta_window.located_on_workspace(
          workspaceManager.get_active_workspace()
        ),
        moveable: focusedWindow.meta_window.allows_move(),
        resizeable: focusedWindow.meta_window.allows_resize(),
        canclose: focusedWindow.meta_window.can_close(),
        canmaximize: focusedWindow.meta_window.can_maximize(),
        maximized: focusedWindow.meta_window.is_maximized(),
        canminimize: focusedWindow.meta_window.can_minimize(),
        display: focusedWindow.meta_window.get_display(),
        frame_type: focusedWindow.meta_window.get_frame_type(),
        window_type: focusedWindow.meta_window.get_window_type(),
        layer: focusedWindow.meta_window.get_layer(),
        monitor: focusedWindow.meta_window.get_monitor(),
        role: focusedWindow.meta_window.get_role(),
        area: validMonitor ? focusedWindow.meta_window.get_work_area_current_monitor() : null,
        area_all: validMonitor ? focusedWindow.meta_window.get_work_area_all_monitors() : null,
        area_cust: validMonitor ? focusedWindow.meta_window.get_work_area_for_monitor(currentmonitor) : null,
      });
    } else {
      return "{}";
    }
  }

  MoveWindow(title, x, y) {
    let window_list = global.get_window_actors();
    let winActor = window_list.find((actor) => {
      let t = actor.meta_window ? actor.meta_window.get_title() : '';
      return t && t.includes(title);
    });
    if (winActor && winActor.meta_window) {
      winActor.meta_window.move_frame(true, x, y);
    }
  }

  #windowTracker = null;
  #focusConnection = null;

  enable() {
    this.#windowTracker = Shell.WindowTracker.get_default();
    this.#focusConnection = this.#windowTracker.connect("notify::focus-app", () => {
      this._dbus.emit_signal("FocusChanged", new GLib.Variant("(s)", [this.Get()]));
    });

    this._dbus = Gio.DBusExportedObject.wrapJSObject(DBUS_SCHEMA, this);
    this._dbus.export(
      Gio.DBus.session,
      "/org/gnome/shell/extensions/FocusedWindow"
    );
  }

  disable() {
    if (this.#windowTracker && this.#focusConnection) {
      this.#windowTracker.disconnect(this.#focusConnection);
      this.#focusConnection = null;
      this.#windowTracker = null;
    }

    if (this._dbus) {
      this._dbus.flush();
      this._dbus.unexport();
      delete this._dbus;
    }
  }
}
