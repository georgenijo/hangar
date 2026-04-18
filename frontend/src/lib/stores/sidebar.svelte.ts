const DASHBOARD_KEY = 'hangar.sidebar.dashboard.collapsed';
const SESSION_KEY = 'hangar.sidebar.session.collapsed';

function readBool(key: string): boolean {
	try {
		if (typeof localStorage === 'undefined') return false;
		return localStorage.getItem(key) === '1';
	} catch {
		return false;
	}
}

function writeBool(key: string, val: boolean) {
	try {
		if (typeof localStorage === 'undefined') return;
		localStorage.setItem(key, val ? '1' : '0');
	} catch {
		// ignore (SSR / privacy mode)
	}
}

let dashboardCollapsed = $state(readBool(DASHBOARD_KEY));
let sessionCollapsed = $state(readBool(SESSION_KEY));

export const sidebarStore = {
	get dashboardCollapsed() {
		return dashboardCollapsed;
	},
	get sessionCollapsed() {
		return sessionCollapsed;
	},
	toggleDashboard() {
		dashboardCollapsed = !dashboardCollapsed;
		writeBool(DASHBOARD_KEY, dashboardCollapsed);
	},
	toggleSession() {
		sessionCollapsed = !sessionCollapsed;
		writeBool(SESSION_KEY, sessionCollapsed);
	},
	setDashboard(val: boolean) {
		dashboardCollapsed = val;
		writeBool(DASHBOARD_KEY, val);
	},
	setSession(val: boolean) {
		sessionCollapsed = val;
		writeBool(SESSION_KEY, val);
	},
};
