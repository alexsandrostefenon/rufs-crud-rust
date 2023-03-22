import {OpenAPI} from '../rufs_crud_rust.js';

import {CaseConvert} from "./CaseConvert.js";
import {DataStoreItem, DataStoreManager} from "./DataStore.js";

class HttpRestRequest {

	constructor(url) {
		if (url.endsWith("/") == true) url = url.substring(0, url.length-1);
		// TODO : change "rest" by openapi.server.base
		this.url = url + "/rest";
		this.messageWorking = "";
		this.messageError = "";
	}

	getToken() {
		return this.token;
	}

	setToken(token) {
		this.token = token;
	}

	static urlSearchParamsToJson(urlSearchParams, properties) {
		const convertSearchParamsTypes = (searchParams, properties) => {
			const reservedParams = ["primaryKey", "overwrite", "filter", "filterRange", "filterRangeMin", "filterRangeMax"];

			for (let name of reservedParams) {
				let obj = searchParams[name];

				if (obj != undefined) {
					for (let [fieldName, value] of Object.entries(obj)) {
						let field = properties[fieldName];

						if (field != undefined) {
							if (field.type == "integer")
								obj[fieldName] = Number.parseInt(value);
							else if (field.type == "number")
								obj[fieldName] = Number.parseFloat(value);
							else if (field.type.startsWith("date") == true)
								obj[fieldName] = new Date(value);
						}
					}
				}
			}
		}

		if (urlSearchParams == undefined || urlSearchParams == null)
			return {};

		const _Qs = HttpRestRequest.Qs != null ? HttpRestRequest.Qs : Qs;
		const searchParams = _Qs.parse(urlSearchParams, {ignoreQueryPrefix: true, allowDots: true});
		if (properties != undefined) convertSearchParamsTypes(searchParams, properties);
		return searchParams;
	}
	// private
	request(path, method, params, objSend) {
		let url = this.url;
		if (url.endsWith("/") == false && path.startsWith("/") == false) url = url + "/";
		url = url + path;

		if (params != undefined && params != null) {
			const _Qs = HttpRestRequest.Qs != null ? HttpRestRequest.Qs : Qs;
			url = url + "?" + _Qs.stringify(params, {allowDots: true});
		}
		
		let options = {};
		options.method = method;
		options.headers = {};

		if (this.token != undefined) {
			options.headers["Authorization"] = "Bearer " + this.token;
		}

		if (objSend != undefined && objSend != null) {
			if (objSend instanceof Map) {
				let obj = {};

				for (let [key, value] of objSend) {
					obj[key] = value;
				}
				
				options.headers["content-type"] = "application/json";
				options.body = JSON.stringify(obj);
			} else if (typeof(objSend) === 'object') {
				options.headers["content-type"] = "application/json";
				options.body = JSON.stringify(objSend);
			} else if (typeof(objSend) === 'string') {
				options.headers["content-type"] = "application/text";
				options.body = objSend;
			} else if (objSend instanceof Blob) {
				options.headers["content-type"] = objSend.type;
				options.body = objSend;
			} else {
				throw new Error("HttpRestRequest.request : unknow data type");
			}
		}

		this.messageWorking = "Processing request to " + url;
		this.messageError = "";
		let promise = Promise.resolve();

		if (this.$scope != undefined) {
			promise = promise.then(() => this.$scope.$apply());
		}

		let _fetch = HttpRestRequest.fetch;
		if (_fetch == undefined) _fetch = fetch;

		if (HttpRestRequest.$q) {
			promise = promise.then(() => HttpRestRequest.$q.when(_fetch(url, options)));
		} else {
			promise = promise.then(() => HttpRestRequest.fetch(url, options));
		}

		return promise.then(response => {
			this.messageWorking = "";
			const contentType = response.headers.get("content-type");
			
			if (response.status === 200) {
				if (contentType) {
					if (contentType.indexOf("application/json") >= 0) {
						return response.json();
					} else if (contentType.indexOf("application/text") >= 0){
						return response.text();
					} else {
						return response.blob();
					}
				} else {
					return Promise.resolve(null);
				}
			} else {
				return response.text().then(message => {
					throw new Error(response.statusText + " : " + message);
				});
			}
		}).catch(error => {
			this.messageError = error.message;
			if (this.$scope != undefined) this.$scope.$apply();
			throw error;
		});
	}

	save(path, itemSend) {
		return this.request(path, "POST", null, itemSend);
	}

	update(path, params, itemSend) {
		return this.request(path, "PUT", params, itemSend);
	}

	patch(path, itemSend) {
		return this.request(path, "PATCH", null, itemSend);
	}

	remove(path, params) {
		return this.request(path, "DELETE", params, null);
	}

	get(path, params) {
		return this.request(path, "GET", params, null).
		then(resp => {
			if (Array.isArray(resp) && resp.length == 1) {
				return resp[0]
			}

			return resp
		});
	}

	query(path, params) {
		return this.request(path, "GET", params, null);
	}

}

class RufsService extends DataStoreItem {

	constructor(serverConnection, path, method, schema_place, property_name, httpRest) {
		super(serverConnection, path, method, schema_place, property_name, []);
		this.serverConnection = serverConnection;
		this.httpRest = httpRest;
        let appName = serverConnection.appName != undefined ? serverConnection.appName : "crud";
	}

	process(action, params) {
		return super.process(action, params).then(() => {
			if (action == "search") {
				if (params.filter != undefined || params.filterRangeMin != undefined || params.filterRangeMax != undefined) {
					return this.queryRemote(params);
				}
			}

			return Promise.resolve();
		})
	}

	request(path, method, params, objSend) {
        return this.httpRest.request(this.path + "/" + path, method, params, objSend);
	}

	get(primaryKey) {
		return this.serverConnection.get(this.name, primaryKey, false);
	}

	save(itemSend) {
		console.log(`[${this.constructor.name}.save()] openapi.copy_fields("${this.path}", "${this.method}", ${this.schema_place}, ${false}, &json!(${JSON.stringify(itemSend)}), ${false}, ${false}, ${false})`);
		const dataOut = this.openapi.copy_fields(this.path, this.method, this.schema_place, false, itemSend, false, false, false);
    	return this.httpRest.save(this.path, dataOut).
    	then(dataIn => {
    		return this.updateList(dataIn);
    	});
	}

	update(primaryKey, itemSend) {
		console.log(`[${this.constructor.name}.update()] openapi.copy_fields("${this.path}", "${this.method}", &SchemaPlace::${this.schema_place}, ${false}, &json!(${JSON.stringify(itemSend)}), ${false}, ${false}, ${false})`);
		const data = this.openapi.copy_fields(this.path, this.method, this.schema_place, false, itemSend, false, false, false);
        return this.httpRest.update(this.path, primaryKey, data).then(data => {
            let pos = this.findPos(primaryKey);
        	return this.updateList(data, pos, pos);
        });
	}

	patch(itemSend) {
    	return this.httpRest.patch(this.path, this.openapi.copy_fields(this.path, this.method, this.schema_place, itemSend)).then(data => this.updateList(data));
	}

	remove(primaryKey) {
        return this.httpRest.remove(this.path, primaryKey);//.then(data => this.serverConnection.removeInternal(this.name, primaryKey));
	}

	queryRemote(params) {
        return this.httpRest.query(this.path, params).then(list => {
			for (let [fieldName, field] of Object.entries(this.properties))
				if (field.type.includes("date") || field.type.includes("time"))
					list.forEach(item => item[fieldName] = new Date(item[fieldName]));
        	this.list = list;
        	return list;
        });
	}

}

class ServerConnection extends DataStoreManager {

	constructor() {
    	super();
    	this.pathname = "";
		this.remoteListeners = [];
	}

	clearRemoteListeners() {
		this.remoteListeners = [];
	}

	addRemoteListener(listenerInstance) {
		this.remoteListeners.push(listenerInstance);
	}

	removeInternal(schemaName, primaryKey) {
		const ret =  super.removeInternal(schemaName, primaryKey);
		for (let listener of this.remoteListeners) listener.onNotify(schemaName, primaryKey, "delete");
		return ret;
	}

	get(schemaName, primaryKey, ignoreCache) {
		return super.get(schemaName, primaryKey, ignoreCache).
		then(res => {
			if (res != null && res != undefined) {
				res.isCache = true;
				return Promise.resolve(res);
			}

			const service = this.getSchema(schemaName);
			if (service == null || service == undefined) return Promise.resolve(null);
			return this.httpRest.get(service.path, primaryKey).
			then(data => {
				if (data == null) return null;
				data.isCache = false;
				return service.cache(primaryKey, data);
			});
		});
	}
	// private -- used in login()
	webSocketConnect(path) {
		// Open a WebSocket connection
		// 'wss://localhost:8443/xxx/websocket'
		var url = this.url;

		if (url.startsWith("https://")) {
			url = "wss://" + url.substring(8);
		} else if (url.startsWith("http://")) {
			url = "ws://" + url.substring(7);
		}

		if (url.endsWith("/") == false) url = url + "/";
		url = url + path;
		if (url.endsWith("/") == false) url = url + "/";
		url = url + "websocket";
		let _WebSocket = ServerConnection.WebSocket;
		if (_WebSocket == undefined) _WebSocket = WebSocket;
		this.webSocket = new _WebSocket(url);

    	this.webSocket.onopen = event => {
    		this.webSocket.send(this.httpRest.getToken());
    	};

    	this.webSocket.onmessage = event => {
			var item = JSON.parse(event.data);
            console.log("[ServerConnection] webSocketConnect : onMessage :", item);
            var service = this.services[item.service];

            if (service != undefined) {
        		if (item.action == "delete") {
        			if (service.findOne(item.primaryKey) != null) {
            			this.removeInternal(item.service, item.primaryKey);
        			} else {
        	            console.log("[ServerConnection] webSocketConnect : onMessage : delete : alread removed", item);
        			}
        		} else {
        			this.get(item.service, item.primaryKey, true).
        			then(res => {
						for (let listener of this.remoteListeners) listener.onNotify(item.service, item.primaryKey, item.action);
						return res;
        			});
        		}
            }
		};
	}
    // public
    login(server, path, loginPath, user, password, RufsServiceClass, callbackPartial) {
		this.url = server;
		if (path != null && path.startsWith("/")) path = path.substring(1);
		if (path != null && path.endsWith("/")) path = path.substring(0, path.length-1);
		if (RufsServiceClass == undefined) RufsServiceClass = RufsService;
		if (callbackPartial == undefined) callbackPartial = console.log;
    	this.httpRest = new HttpRestRequest(this.url);
    	return this.httpRest.request(loginPath, "POST", null, {"user":user, "password":password}).
    	then(loginResponse => {
			this.openapi = OpenAPI.from_str(JSON.stringify(loginResponse.openapi));
    		this.title = loginResponse.title;
			this.rufsGroupOwner = loginResponse.rufsGroupOwner;
			this.routes = loginResponse.routes;
			this.path = loginResponse.path;
			this.userMenu = loginResponse.menu;
    		this.httpRest.setToken(loginResponse.jwtHeader);
    		const services = [];
            let listDependencies = [];
            // depois carrega os serviços autorizados
			for (let role of loginResponse.roles) {
				const schemaName = CaseConvert.underscoreToCamel(role.path.substring(1))
				const service = this.services[schemaName] = new RufsServiceClass(this, role.path, "get", "response", null, this.httpRest);
				service.access = {};
				service.params = {};
				const methods = ["get", "post", "put", "delete"];

				for (let i = 0; i < methods.length; i++) {
					const method = methods[i]

					if ((role.mask & (1 << i)) != 0)
						service.access[method] = true;
					else
						service.access[method] = false;
				}

				if (service.properties.rufsGroupOwner != null && this.rufsGroupOwner != 1) {
					service.properties.rufsGroupOwner.hiden = true;
				}
				
				if (service.properties.rufsGroupOwner != undefined && service.properties.rufsGroupOwner.default == undefined) {
					service.properties.rufsGroupOwner.default = this.rufsGroupOwner;
				}
				
				services.push(service);
				Array.prototype.push(listDependencies, this.openapi.get_dependencies(schemaName));

				if (listDependencies.includes(schemaName) == false) {
					listDependencies.push(schemaName);
				}
			}

            this.setSchemas(services, this.openapi);
//    		if (user == "admin") listDependencies = ["rufsUser", "rufsGroupOwner", "rufsGroup", "rufsGroupUser"];
    		const listQueryRemote = [];

    		for (let schemaName of listDependencies) {
				console.log(`login ${schemaName}`)
    			const service = this.getSchema(schemaName);

				if (service != null && service.access.get == true) {
					listQueryRemote.push(service);
				}
    		}

            return new Promise((resolve, reject) => {
            	var queryRemoteServices = () => {
            		if (listQueryRemote.length > 0) {
            			let service = listQueryRemote.shift();
                		console.log("[ServerConnection] loading", service.label, "...");
                		callbackPartial("loading... " + service.label);

                		service.queryRemote(null).then(list => {
                			console.log("[ServerConnection] ...loaded", service.label, list.length);
                			queryRemoteServices();
                		}).catch(error => reject(error));
            		} else {
               			console.log("[ServerConnection] ...loaded services");
                    	resolve(loginResponse);
            		}
            	}

                queryRemoteServices();
        	}).then(loginResponse => {
				this.webSocketConnect(path);
				return loginResponse;
			});
    	}).catch(err => {
			console.error(err);
			throw(err);
		});
    }
    // public
    logout() {
		this.webSocket.close();
   		this.httpRest.setToken(undefined);
        // limpa todos os dados da sessão anterior
        for (let serviceName in this.services) {
        	delete this.services[serviceName];
        }
    }

}

HttpRestRequest.MD5 = (d) => {var r = M(V(Y(X(d),8*d.length)));return r.toLowerCase()};function M(d){for(var _,m="0123456789ABCDEF",f="",r=0;r<d.length;r++)_=d.charCodeAt(r),f+=m.charAt(_>>>4&15)+m.charAt(15&_);return f}function X(d){for(var _=Array(d.length>>2),m=0;m<_.length;m++)_[m]=0;for(m=0;m<8*d.length;m+=8)_[m>>5]|=(255&d.charCodeAt(m/8))<<m%32;return _}function V(d){for(var _="",m=0;m<32*d.length;m+=8)_+=String.fromCharCode(d[m>>5]>>>m%32&255);return _}function Y(d,_){d[_>>5]|=128<<_%32,d[14+(_+64>>>9<<4)]=_;for(var m=1732584193,f=-271733879,r=-1732584194,i=271733878,n=0;n<d.length;n+=16){var h=m,t=f,g=r,e=i;f=md5_ii(f=md5_ii(f=md5_ii(f=md5_ii(f=md5_hh(f=md5_hh(f=md5_hh(f=md5_hh(f=md5_gg(f=md5_gg(f=md5_gg(f=md5_gg(f=md5_ff(f=md5_ff(f=md5_ff(f=md5_ff(f,r=md5_ff(r,i=md5_ff(i,m=md5_ff(m,f,r,i,d[n+0],7,-680876936),f,r,d[n+1],12,-389564586),m,f,d[n+2],17,606105819),i,m,d[n+3],22,-1044525330),r=md5_ff(r,i=md5_ff(i,m=md5_ff(m,f,r,i,d[n+4],7,-176418897),f,r,d[n+5],12,1200080426),m,f,d[n+6],17,-1473231341),i,m,d[n+7],22,-45705983),r=md5_ff(r,i=md5_ff(i,m=md5_ff(m,f,r,i,d[n+8],7,1770035416),f,r,d[n+9],12,-1958414417),m,f,d[n+10],17,-42063),i,m,d[n+11],22,-1990404162),r=md5_ff(r,i=md5_ff(i,m=md5_ff(m,f,r,i,d[n+12],7,1804603682),f,r,d[n+13],12,-40341101),m,f,d[n+14],17,-1502002290),i,m,d[n+15],22,1236535329),r=md5_gg(r,i=md5_gg(i,m=md5_gg(m,f,r,i,d[n+1],5,-165796510),f,r,d[n+6],9,-1069501632),m,f,d[n+11],14,643717713),i,m,d[n+0],20,-373897302),r=md5_gg(r,i=md5_gg(i,m=md5_gg(m,f,r,i,d[n+5],5,-701558691),f,r,d[n+10],9,38016083),m,f,d[n+15],14,-660478335),i,m,d[n+4],20,-405537848),r=md5_gg(r,i=md5_gg(i,m=md5_gg(m,f,r,i,d[n+9],5,568446438),f,r,d[n+14],9,-1019803690),m,f,d[n+3],14,-187363961),i,m,d[n+8],20,1163531501),r=md5_gg(r,i=md5_gg(i,m=md5_gg(m,f,r,i,d[n+13],5,-1444681467),f,r,d[n+2],9,-51403784),m,f,d[n+7],14,1735328473),i,m,d[n+12],20,-1926607734),r=md5_hh(r,i=md5_hh(i,m=md5_hh(m,f,r,i,d[n+5],4,-378558),f,r,d[n+8],11,-2022574463),m,f,d[n+11],16,1839030562),i,m,d[n+14],23,-35309556),r=md5_hh(r,i=md5_hh(i,m=md5_hh(m,f,r,i,d[n+1],4,-1530992060),f,r,d[n+4],11,1272893353),m,f,d[n+7],16,-155497632),i,m,d[n+10],23,-1094730640),r=md5_hh(r,i=md5_hh(i,m=md5_hh(m,f,r,i,d[n+13],4,681279174),f,r,d[n+0],11,-358537222),m,f,d[n+3],16,-722521979),i,m,d[n+6],23,76029189),r=md5_hh(r,i=md5_hh(i,m=md5_hh(m,f,r,i,d[n+9],4,-640364487),f,r,d[n+12],11,-421815835),m,f,d[n+15],16,530742520),i,m,d[n+2],23,-995338651),r=md5_ii(r,i=md5_ii(i,m=md5_ii(m,f,r,i,d[n+0],6,-198630844),f,r,d[n+7],10,1126891415),m,f,d[n+14],15,-1416354905),i,m,d[n+5],21,-57434055),r=md5_ii(r,i=md5_ii(i,m=md5_ii(m,f,r,i,d[n+12],6,1700485571),f,r,d[n+3],10,-1894986606),m,f,d[n+10],15,-1051523),i,m,d[n+1],21,-2054922799),r=md5_ii(r,i=md5_ii(i,m=md5_ii(m,f,r,i,d[n+8],6,1873313359),f,r,d[n+15],10,-30611744),m,f,d[n+6],15,-1560198380),i,m,d[n+13],21,1309151649),r=md5_ii(r,i=md5_ii(i,m=md5_ii(m,f,r,i,d[n+4],6,-145523070),f,r,d[n+11],10,-1120210379),m,f,d[n+2],15,718787259),i,m,d[n+9],21,-343485551),m=safe_add(m,h),f=safe_add(f,t),r=safe_add(r,g),i=safe_add(i,e)}return Array(m,f,r,i)}function md5_cmn(d,_,m,f,r,i){return safe_add(bit_rol(safe_add(safe_add(_,d),safe_add(f,i)),r),m)}function md5_ff(d,_,m,f,r,i,n){return md5_cmn(_&m|~_&f,d,_,r,i,n)}function md5_gg(d,_,m,f,r,i,n){return md5_cmn(_&f|m&~f,d,_,r,i,n)}function md5_hh(d,_,m,f,r,i,n){return md5_cmn(_^m^f,d,_,r,i,n)}function md5_ii(d,_,m,f,r,i,n){return md5_cmn(m^(_|~f),d,_,r,i,n)}function safe_add(d,_){var m=(65535&d)+(65535&_);return(d>>16)+(_>>16)+(m>>16)<<16|65535&m}function bit_rol(d,_){return d<<_|d>>>32-_}

export {HttpRestRequest, RufsService, ServerConnection};
