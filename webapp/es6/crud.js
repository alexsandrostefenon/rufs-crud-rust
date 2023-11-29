import {ServerConnectionUI} from "./ServerConnectionUI.js";
import {CrudController} from "./CrudController.js";

class ServerConnectionService extends ServerConnectionUI {

	constructor($locale, $route, $rootScope, $q, $timeout, $controllerProvider, $routeProvider) {
		super($locale, $route, $rootScope, $q, $timeout, $controllerProvider, $routeProvider);
    }

    login(server, path, loginPath, user, password, callbackPartial) {
        return super.login(server, path, loginPath, user, password, CrudServiceUI, callbackPartial);
    }

}

class LoginController {

    constructor(serverConnection, server, $scope) {
		this.serverConnection = serverConnection;
		this.server = server;
		this.$scope = $scope;
//		this.loginPath = "base/rest/login";
		this.user = "";
		this.password = "";
		this.message = "";

	  	if (this.serverConnection.httpRest != null && this.serverConnection.httpRest.token != null) {
	    	this.serverConnection.logout();
	    	window.location.reload();
		}
    }

    login(loginPath) {
    	// TODO : resolve path to load from UI
    	if (loginPath == undefined) loginPath = this.loginPath;
    	this.path = "";
    	return this.serverConnection.login(this.server, this.path, loginPath, this.user, HttpRestRequest.MD5(this.password), message => this.message = message).
    	catch(res => {
			this.$scope.$apply();
			return res;
    	});
    }
}

class MenuController {

    constructor(serverConnection) {
    	this.serverConnection = serverConnection;
    	this.isCollapsed = true;
    }

}

class Crud {
	
    static initialize($controllerProvider, $routeProvider, $compileProvider, $provide) {
    	$provide.service("ServerConnectionService", function($locale, $route, $rootScope, $q, $timeout) {
    		HttpRestRequest.$q = $q;
    		return new ServerConnectionService($locale, $route, $rootScope, $q, $timeout, $controllerProvider, $routeProvider);
    	});

    	$controllerProvider.register("CrudController", function(ServerConnectionService, $scope) {
    		return new CrudController(ServerConnectionService, $scope);
    	});

    	$controllerProvider.register('LoginController', function(ServerConnectionService, $scope) {
    		const url = new URL(window.location.hash.substring(2), window.location.href);
    		const server = url.searchParams.get("server");
    		console.log("Crud.initialize : LoginController.server = ", server);
    		return new LoginController(ServerConnectionService, server, $scope);
    	});

    	$controllerProvider.register("MenuController", function(ServerConnectionService) {
    	    return new MenuController(ServerConnectionService);
    	});

    	$compileProvider.directive('crudTable', () => {
    		return {restrict: 'E', scope: {vm: '=crud'}, templateUrl: './templates/crud-table.html'};
    	});

    	$compileProvider.directive('crudItem', () => {
    		return {restrict: 'E', scope: {vm: '=', edit: '='}, templateUrl: './templates/crud-item.html'};
    	});

    	$compileProvider.directive('crudItemJson', () => {
    		return {restrict: 'E', scope: {vm: '=', edit: '='}, templateUrl: './templates/crud-item-json.html'};
    	});

    	$compileProvider.directive('crudJsonArray', () => {
    		return {restrict: 'E', scope: {vm: '=', edit: '='}, templateUrl: './templates/crud-json-array.html'};
    	});

    	$compileProvider.directive('crudObjJson', () => {
    		return {restrict: 'E', scope: {vm: '=', edit: '='}, templateUrl: './templates/crud-obj-json.html'};
    	});
    }
    
}

export {Crud};

