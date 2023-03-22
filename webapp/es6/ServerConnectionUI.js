import {RufsService, ServerConnection, HttpRestRequest} from "./ServerConnection.js";
import {RufsSchema} from "./DataStore.js";
import {CrudController} from "./CrudController.js";

class CrudServiceUI extends RufsService {

	constructor(serverConnection, path, method, schema_place, property_name, httpRest) {
		super(serverConnection, path, method, schema_place, property_name, httpRest);
		const paths = serverConnection.openapi.get_paths();
		const pathItemObject = paths[path];
		this.label = pathItemObject.summary || serverConnection.convertCaseAnyToLabel(this.path);
		this.listStr = [];
	}
	// private
    buildItemStr(item) {
		let stringBuffer = [];
		let str = "";

		for (let fieldName of this.shortDescriptionList) {
			this.buildField(stringBuffer, fieldName, item);
		}

		if (stringBuffer.length > 0) str = stringBuffer.join(" - ");
		return str;
    }

    buildListStr(list) {
		console.time("buildListStr : " + this.label);
    	var ret = [];

    	for (var i = 0; i < list.length; i++) {
    		const item = list[i];
    		const str = this.buildItemStr(item);
			const pos = ret.indexOf(str);

			if (pos >= 0) {
//				console.error(`[${this.constructor.name}.buildListStr(${this.name})] : already exists string in list : oldPos = ${pos}, newPos = ${i}, str = ${str})`, list[pos], list[i]);
//				console.trace();
//				debugger;
				this.buildItemStr(item);
			}

    		ret.push(str);
    	}

		console.timeEnd("buildListStr : " + this.label);
    	return ret;
    }
	// private, params.data, params.oldPos, params.newPos
	updateListStr(params) {
		if (params == null) {
			console.error("CrudServiceUI.updateListStr : received null parameter");
			return null;
		}

		const assertExists = (list, str, action, params) => {
			const pos = list.indexOf(str);

			if (pos >= 0 && pos != params.newPos) {
				console.error(`[${this.constructor.name}.updateListStr(${this.name}, ${action})] : already exists string in listStr : list.length = ${list.length}, pos = ${pos}, oldPos = ${params.oldPos}, newPos = ${params.newPos}, str = ${str})`);
				console.trace();
//				debugger;
			}
		}

		let str = params.data == undefined ? null : this.buildItemStr(params.data);

        if (params.oldPos == undefined) {
        	// add
        	assertExists(this.listStr, str, "add", params);
        	this.listStr.push(str);
        } else if (params.newPos == undefined) {
        	// remove
        	this.listStr.splice(params.oldPos, 1);
//			console.log(`[${this.constructor.name}.updateListStr(${this.name}, ${params.oldPos}, ${params.newPos})] remove :`, str);
        } else if (params.newPos == params.oldPos) {
        	// replace
        	assertExists(this.listStr, str, "replace", params);
        	this.listStr[params.newPos] = str;
//			console.log(`[${this.constructor.name}.updateListStr(${this.name}, ${params.oldPos}, ${params.newPos})] replace :`, str);
        } else if (params.newPos != undefined) {
        	// remove and add
        	this.listStr.splice(params.oldPos, 1);
        	assertExists(this.listStr, str, "remove and add", params);
        	this.listStr.splice(params.newPos, 0, str);
        }
        
        return params;
	}

	get(primaryKey) {
    	return super.get(primaryKey);
	}

	save(itemSend) {
    	return super.save(itemSend).then(response => this.updateListStr(response));
	}

	update(primaryKey, itemSend) {
        return super.update(primaryKey, itemSend).then(response => this.updateListStr(response));
	}

	remove(primaryKey) {
        // data may be null
    	return super.remove(primaryKey);//.then(response => this.updateListStr(response));
	}

	queryRemote(params) {
    	return super.queryRemote(params).then(list => {
    		this.listStr = this.buildListStr(this.list);
    		const dependents = this.openapi.get_dependents(this.name, false);
    		const listProcessed = [];
            // também atualiza a lista de nomes de todos os serviços que dependem deste
			for (let item of dependents) {
				if (listProcessed.includes(item.schema) == false) {
					let service = this.dataStoreManager.services[item.schema];

					if (service == null) {
						continue;
					}
					
					service.listStr = service.buildListStr(service.list);
					listProcessed.push(item.schema);
				}
			}

			return list;
    	});
	}

}

class ServerConnectionUI extends ServerConnection {

	static buildLocationHash(hashPath, hashSearchObj) {
		if (hashPath.startsWith("/") == false) {
			hashPath = "/" + hashPath;
		}
		
		let hash = "#!/app" + hashPath;

		if (hashSearchObj != undefined) {
			hash = hash + "?" + Qs.stringify(hashSearchObj, {allowDots: true});
		}
		
		return hash;
	}

	static changeLocationHash(hashPath, hashSearchObj) {
		const hash = ServerConnectionUI.buildLocationHash(hashPath, hashSearchObj);
		console.log(`ServerConnectionUI.changeLocationHash(${hashPath}, ${hashSearchObj}) : ${hash}`);
		window.location.assign(hash);
	}

	constructor($locale, $route, $rootScope, $q, $timeout, $controllerProvider, $routeProvider) {
		super();
    	this.localeId = $locale.id;
    	this.$route = $route;
    	this.$rootScope = $rootScope;
    	this.$q = $q;
    	this.$timeout = $timeout;
    	this.$controllerProvider = $controllerProvider;
    	this.$routeProvider = $routeProvider;
    	// força o browser a iniciar sempre da página de login
    	if (window.location.hash.indexOf("/login") < 0) {
			ServerConnectionUI.changeLocationHash("login");
    	}

		this.translation = {};
		this.translation.new = "New";
		this.translation.exit = "Exit";
		this.translation.clear = "Limpar";
		this.translation.saveAsNew = "Save as New";
		this.translation.view = "View";
		this.translation.edit = "Edit";
		this.translation.delete = "Delete";
		this.translation.create = "Create";
		this.translation.search = "Search";
		this.translation.cancel = "Cancel";
		this.translation.save = "Save";
		this.translation.filter = "Filter";
        this.menu = [];
	}
	// used by websocket
	removeInternal(schemaName, primaryKey) {
		const schema = this.getSchema(schemaName);
		if (schema == undefined) return undefined;
		let response = super.removeInternal(schemaName, primaryKey);

		if (response != null && response != undefined) {
			console.log("CrudServiceUI.removeInternal : doing updateListStr :", schema.listStr[response.oldPos]);
	        return schema.updateListStr(response);
		} else {
			console.log("CrudServiceUI.removeInternal : alread removed, primaryKey = ", primaryKey);
			return null;
		}
	}

	get(schemaName, primaryKey, ignoreCache) {
		return super.get(schemaName, primaryKey, ignoreCache).
		then(response => {
			if (response.isCache == true) return response;
			const service = this.getSchema(schemaName);
			return service.updateListStr(response);
		});
	}

	getDocument(service, obj, merge, tokenData) {
		return super.getDocument(service, obj, merge, tokenData).then(() => {
			if (service.primaryKeys.length > 0) {
				const primaryKey = service.getPrimaryKey(obj);

				if (primaryKey != null) {
					const pos = service.findPos(primaryKey);

					if (pos >= 0) {
						if (service.updateListStr != undefined) {
							service.updateListStr({data: obj, oldPos: pos, newPos: pos});
						} else {
							console.error(`[${this.constructor.name}.getDocument()] : missing updateListStr`);
						}
					}
				}
			}
		});
	}

    // private <- login
	loginDone() {
        this.menu = [{"group": "Geral", items: [{path:"login", label:"Exit"}]}];
        // user menu
		if (this.userMenu != undefined) {
			for (let menuItem of this.userMenu) {
				let group = this.menu.find(item => item.group == menuItem.group);

				if (group == null) {
					group = {"group": menuItem.group, "items": []};
					this.menu.push(group);
				}

				group.items.push(menuItem);
			}
		}
    	// tradução
		if (this.services.rufsTranslation != undefined) {
	    	for (var fieldName in this.translation) {
	    		const str = this.translation[fieldName];
		    	const item = this.services.rufsTranslation.findOne({name:str,locale:this.localeId});

	    		if (item != null && item.translation != undefined) {
	        		this.translation[fieldName] = item.translation;
	    		}
	    	}
		}
        // routes and modules
//		var config = JSON.parse(this.config);
		const promises = [];
		this.$routeProvider.when('/app/login',{templateUrl:"./templates/login.html", controller:'LoginController', controllerAs: "vm"});

		if (this.routes != undefined && this.routes != null) {
			let routes = [];

			if (Array.isArray(this.routes) == true) {
				routes = this.routes;
			} else if (typeof this.routes === 'string' || this.routes instanceof String) {
				try {
					routes = JSON.parse(this.routes);
				} catch (e) {
					console.error("fail to parse json from string this.routes : ", this.routes, "err : ", e);
				}
			} else {
				console.error("invalid routes");
			}

			for (let route of routes) {
				if (route.templateUrl == undefined || route.templateUrl.length == 0) {
					route.templateUrl = "./templates/crud.html";
				}

				let path = route.controller;
				if (path.startsWith("/") == false && path.startsWith(".") == false) path = "./" + path;
				if (path.endsWith(".js") == false) path = path + ".js";
				console.log("loading...", path);

				let promise = import(path).
				catch(err => {
					console.error(`[${this.constructor.name}.loginDone()] : fail to import module :`, err);
					throw err;
				}).
				then(module => {
					const controllerName = route.controller;
					console.log("...loaded:", controllerName, "route:", route);

					this.$controllerProvider.register(controllerName, function(ServerConnectionService, $scope) {
						const _class = module[controllerName];
						return new _class(ServerConnectionService, $scope);
					});

					this.$routeProvider.when(route.path, {"templateUrl":route.templateUrl, "controller": controllerName, controllerAs: "vm"});
				});
				
				promises.push(promise);
			}
		}

		return Promise.all(promises).then(() => {
			console.log("Promise.all:", promises);
			this.$routeProvider.when("/app/:name/:action", {templateUrl: "./templates/crud.html", controller: "CrudController", controllerAs: "vm"});
			this.$routeProvider.otherwise({redirectTo: '/app/login'});

	        if (this.path != undefined && this.path != null && this.path.length > 0) {
	        	this.$route.reload();
	        	ServerConnectionUI.changeLocationHash(this.path);
	        }
		});
    }
    // public
    login(server, path, loginPath, user, password, RufsServiceClass, callbackPartial) {
    	if (server == null || server.length == 0) {
    		server = window.location.origin;
    	}

    	if (path == null || path.length == 0) {
    		path = window.location.pathname;
    	}

    	if (loginPath == null || loginPath.length == 0) {
    		loginPath = window.location.pathname;
    		if (loginPath.endsWith("/") == false) loginPath = loginPath + "/";
    		loginPath = loginPath + "login";
    	}

        return super.login(server, path, loginPath, user, password, RufsServiceClass, callbackPartial).then(loginResponse => this.loginDone());
    }

    logout() {
    	super.logout();
        this.menu = [];
		ServerConnectionUI.changeLocationHash("login", {"server":this.url});
    }

	convertCaseAnyToLabel(str) {
		if (str == undefined) {
			console.error(`ServerConnectionUI.convertCaseAnyToLabel(${str})`);
			return "";
		}

		var ret = "";
		var nextIsUpper = true;

		for (var i = 0; i < str.length; i++) {
			var ch = str[i];

			if (nextIsUpper == true) {
				ret = ret + ch.toUpperCase();
				nextIsUpper = false;
			} else if (ch >= 'A' && ch <= 'Z') {
				ret = ret + ' ' + ch;
			} else if (ch == '-' || ch == '_') {
				ret = ret + ' ';
				nextIsUpper = true;
			} else {
				ret = ret + ch;
			}
		}

		if (this.services.rufsTranslation != undefined) {
	    	var item = this.services.rufsTranslation.findOne({name:str,locale:this.localeId});

	    	if (item != null && item.translation != null && item.translation != undefined) {
	    		ret = item.translation;
	    	}
		}

		return ret;
	}

}

export {CrudServiceUI, ServerConnectionUI}
