import {CaseConvert} from "./CaseConvert.js";

class RufsSchema {

	static getMaxFieldSize(schema, fieldName) {
		let ret = 0;
		const field = schema.properties[fieldName];
		const type = field["type"];

		if (type == undefined || type == "string") {
			if (field.maxLength != undefined) {
				ret = field.maxLength;
			} else {
				ret = 100;
			}
		} else if (type == "integer") {
			ret = 9;
		} else if (type == "number") {
			if (field.precision != undefined) {
				ret = field.precision;
			} else {
				ret = 15;
			}
		} else if (type == "boolean") {
			ret = 5;
		} else if (type == "date" || field.format == "date-time") {
			ret = 30;
		}

		return ret;
	}

	setSchema(method, schema_place, property_name) {
		try {
			console.log(`[RufsSchema.setSchema(method, schema_place)] : this.openapi.get_schema(${this.path}, ${method}, ${schema_place})`);
			// TODO : adicionar o paraâmetro "property_name" no Rust.
			this.schema = this.openapi.get_schema(this.path, method, schema_place);
			this.properties = this.schema.properties || this.schema.items.properties;

			if (property_name != null) {
				const property = this.properties[property_name];

				if (property.type == "object") {
					this.schema = property;
					this.properties = this.schema.properties;
				} else if (property.type == "array") {
					this.schema = property.items;
					this.properties = this.schema.properties;
				}
			}
			
			this.foreignKeys = this.schema["x-foreignKeys"] || {};
			this.uniqueKeys = this.schema["x-uniqueKeys"] || {};
			this.primaryKeys = this.schema["x-primaryKeys"] || [];

			if (this.schema.properties["rufsGroupOwner"] != null) {
				this.schema.properties["rufsGroupOwner"]["hiden"] = true;
			}
			
			const entries = Object.entries(this.properties);
			this.shortDescriptionList = [];
	
			for (let [fieldName, field] of entries) {
				if (field.type == null) field.type = "string";
				if (field.orderIndex == null) field.orderIndex = entries.length;
				if (field.tableVisible == null) field.tableVisible = true;
				if (field.hiden == true) field.tableVisible = false;
				if (field.shortDescription == null) field.shortDescription = false;
				if (field.shortDescription == true) this.shortDescriptionList.push(fieldName);
				
				if (field.$ref == null) {
					field.$ref = field["x-$ref"];
				}
				
				if (field.flags == null) {
					field.flags = field["x-flags"];
				}
			}
			// Se não foi definido manualmente o shortDescriptionList, monta em modo automático usando os uniqueMaps
			if (this.shortDescriptionList.length == 0) {
				let shortDescriptionListSize = 0;
	
				for (let [fieldName, field] of entries) {
					if (field.hiden == null && field["x-identityGeneration"] != null && Object.entries(this.uniqueKeys).length > 0) {
						field.hiden = true
						field.tableVisible = false
					}
				}
	
				if (this.primaryKeys.find(fieldName => this.properties[fieldName].tableVisible == false) == undefined) {
					Array.prototype.push.apply(this.shortDescriptionList, this.primaryKeys);
					
					for (let fieldName of this.primaryKeys) {
						shortDescriptionListSize += RufsSchema.getMaxFieldSize(this.schema, fieldName);
					}
				}
	
				for (let [name, list] of Object.entries(this.uniqueKeys)) {
					if (list.find(fieldName => this.properties[fieldName].tableVisible == false) == undefined) {
						for (let fieldName of list) if (this.shortDescriptionList.includes(fieldName) == false) this.shortDescriptionList.push(fieldName);
						if (this.shortDescriptionList.length > 3 || shortDescriptionListSize > 30) break;
					}
				}
	
				for (let [fieldName, field] of entries) {
					if (this.shortDescriptionList.length > 3 || shortDescriptionListSize > 30) break;
	
					if (field.tableVisible == true && this.shortDescriptionList.includes(fieldName) == false) {
						this.shortDescriptionList.push(fieldName);
						shortDescriptionListSize += RufsSchema.getMaxFieldSize(this.schema, fieldName);
					}
				}
			}
		} catch (err) {
			console.error(err);
			throw err;
		}
	}

	constructor(openapi, path, method, schema_place, property_name) {
		this.name = CaseConvert.underscoreToCamel(path.substring(1));
		this.openapi = openapi;
		this.path = path;
		this.method = method;
		this.schema_place = schema_place;
		this.property_name = property_name;
		this.setSchema(method, schema_place, property_name);
	}

	checkPrimaryKey(obj) {
		var check = true;

		for (var fieldName of this.primaryKeys) {
			if (obj[fieldName] == undefined) {
				check = false;
				break;
			}
		}

		return check;
	}
	// private, projected for extract primaryKey and uniqueKeys
	static copyFieldsFromList(dataIn, fieldNames, retutnNullIfAnyEmpty) {
		let ret = {};

		for (let fieldName of fieldNames) {
			if (dataIn[fieldName] != undefined) {
				ret[fieldName] = dataIn[fieldName];
			} else {
				if (retutnNullIfAnyEmpty == true) {
					ret = null;
					break;
				}
			}
		}

		return ret;
	}

	getPrimaryKey(obj) {
		return RufsSchema.copyFieldsFromList(obj, this.primaryKeys, true);
	}
	// public, return primary and uniqueKeys if present in obj
	getKeys(obj) {
		const ret = [];
		// first, primary key
		{
			const primaryKey = this.getPrimaryKey(obj);
			if (primaryKey != null) ret.push(primaryKey);
		}
		// unique keys
		for (let [name, uniqueKey] of Object.entries(this.uniqueKeys)) {
			let key = RufsSchema.copyFieldsFromList(obj, uniqueKey, true);
			if (key != null) ret.push(key);
		}

		return ret;
	}

}
// minimal wrapper to Html5 IndexedDb
class DataStore extends RufsSchema {

	constructor(openapi, path, method, schema_place, property_name, list) {
		super(openapi, path, method, schema_place, property_name);
		this.list = list || [];
	}

	clear() {
		for (let fieldName in this.properties) {
			let field = this.properties[fieldName];
			delete field.externalReferencesStr;
		}

		return Promise.resolve();
	}

	process(action, params) {
		console.log(`${this.constructor.name}.process(${action}, ${JSON.stringify(params)})`);
		return this.clear();
	}

	find(params) {
        return Filter.find(this.list, params);
	}

	findOneIn(paramsList) {
        return Filter.findOneIn(this.list, paramsList);
	}

	findPos(params) {
		return Filter.findPos(this.list, params);
	}

	findOne(params) {
		let pos = this.findPos(params);
		return pos >= 0 ? this.list[pos] : null;
	}
	// private, use in get, save, update and remove
	updateList(data, oldPos, newPos) {
        if (oldPos == undefined && newPos == undefined) {
			let primaryKey = this.getPrimaryKey(data);
			let pos = -1;

			if (primaryKey != null && Object.entries(primaryKey).length > 0) {
				pos = this.findPos(primaryKey);
			}

			if (pos >= 0) {
				oldPos = newPos = pos;
				// replace
				this.list[oldPos] = data;
			} else {
				// add
				this.list.push(data);
				newPos = this.list.length - 1;
			}
        } else if (oldPos != undefined && newPos == undefined) {
        	// remove
        	this.list.splice(oldPos, 1);
        } else if (oldPos != undefined && oldPos == newPos) {
        	// replace
           	this.list[oldPos] = data;
        }
        
        return {"data": data, "oldPos": oldPos, "newPos": newPos};
	}

	cache(primaryKey, data) {
		for (let [fieldName, field] of Object.entries(this.properties)) if (field.type.includes("date") || field.type.includes("time")) data[fieldName] = new Date(data[fieldName]);
		let pos = this.findPos(primaryKey);
		let ret;

		if (pos < 0) {
			ret = this.updateList(data);
		} else {
			ret = this.updateList(data, pos, pos);
		}

//		console.log(`[${this.constructor.name}.cache(${JSON.stringify(primaryKey)})][${this.name}] :`, ret);
		return ret;
	}

}
// manager of  IndexedDb collections
class DataStoreManager {

	setSchemas(list, openapi) {
		const removeBrokenRefs = (schema, openapi) => {
			for (let [fieldName, field] of Object.entries(schema.properties)) {
				if (field.$ref == null) {
					field.$ref = field["x-$ref"];
				}
				
				if (field.$ref != null) {
					const $ref = this.openapi.get_schema_name_from_ref(field.$ref);

					if (this.services[$ref] == undefined) {
						console.error(`[${this.constructor.name}.setSchemas] : missing this.services of ${$ref}`);
						delete field.$ref;
					}

					if (openapi != undefined && openapi.components.schemas[$ref] == undefined) {
						console.error(`[${this.constructor.name}.setSchemas] : missing openapi.components.schemas of ${$ref}`);
						delete field.$ref;
					}
				}
			}
		}

		this.openapi = openapi;
		// TODO : trocar o uso de services por openapi.paths
		this.services = {};

		if (Array.isArray(list) == true) {
			for (let service of list) {
				this.services[service.name] = service;
			}
		}
/*
		for (let [name, schema] of Object.entries(this.services)) {
			removeBrokenRefs(schema, openapi);
		}

		if (openapi == undefined) return;

		for (let [name, schema] of Object.entries(openapi.components.schemas)) {
			removeBrokenRefs(schema, openapi);
		}

		for (let [name, requestBody] of Object.entries(openapi.components.requestBodies)) {
			for (const [mediaTypeName, mediaTypeObject] of Object.entries(requestBody.content)) {
				if (mediaTypeObject.schema.properties != undefined) {
					removeBrokenRefs(mediaTypeObject.schema, openapi);
				}
			}
		}
  */
	}

	constructor(list, openapi) {
		this.setSchemas(list, openapi);
	}

	getSchema(schemaName, tokenPayload) {
		const serviceName = this.openapi.get_schema_name_from_ref(schemaName);
		const path = "/" + CaseConvert.camelToUnderscore(schemaName, false);

		if (tokenPayload && tokenPayload.roles && tokenPayload.roles.find(item => item.path == path) == null) {
			throw new Error(`Unauthorized service Access : ${path}`);
		}
		
		const service = this.services[serviceName];
		return service;
	}
	// used by websocket
	removeInternal(schemaName, primaryKey) {
		const service = this.getSchema(schemaName);
        let pos = service.findPos(primaryKey);
		console.log("DataStore.removeInternal : pos = ", pos, ", data :", service.list[pos]);
        return pos >= 0 ? service.updateList(service.list[pos], pos) : null;
	}
	// ignoreCache is used in websocket notifications
	get(schemaName, primaryKey, ignoreCache) {
		if (ignoreCache == true) {
        	return Promise.resolve(null);
		}

		const dataStore = this.getSchema(schemaName);
        const pos = dataStore.findPos(primaryKey);
        if (pos >= 0) 
        	return Promise.resolve({"data": dataStore.list[pos]});
        else
        	return Promise.resolve(null);
	}

	getDocument(service, obj, merge, tokenPayload) {
		const getPrimaryKeyForeignList = (schema, obj) => {
			const list = [];

			for (let [fieldName, field] of Object.entries(schema.properties)) {
				if (field.$ref != undefined) {
					const item = this.getPrimaryKeyForeign(schema, fieldName, obj);

					if (item.valid == true && list.find(candidate => candidate.fieldName == fieldName) == undefined) {
						list.push({"fieldName": fieldName, item});
					}
				}
			}

			return list;
		}

		let document;

		if (merge != true) {
			document = {};
		} else {
			document = obj;
		}

		let promises = [];
		// One To One
		{
			const next = (document, list) => {
				if (list.length == 0) return;
				const data = list.shift();
				const schemaRef = this.getSchema(data.item.schema);
				
				if (schemaRef == undefined) {
					console.error(data);
					this.getSchema(data.item.schema);
				}

				let promise;

				if (Object.entries(data.item.primary_key).length > 0) {
					promise = this.get(schemaRef.name, data.item.primary_key);
				} else {
					promise = Promise.resolve({});
				}

				
				return promise.
				then(objExternal => document[data.fieldName] = objExternal).
				catch(err => console.error(err)).
//				then(() => next(document, list));
				finally(() => next(document, list));
			}

			const listToGet = getPrimaryKeyForeignList(service, obj);
			promises.push(next(document, listToGet));
		}
		// One To Many
		{
			const dependents = this.openapi.get_dependents(service.name, true, this.services);

			for (let item of dependents) {
				const rufsServiceOther = this.getSchema(item.schema, tokenPayload);
				if (rufsServiceOther == null) continue;
				let field = rufsServiceOther.properties[item.field];
				let foreignKey = Object.fromEntries(this.openapi.get_foreign_key(rufsServiceOther.name, item.field, obj));
				// TODO : check to findRemote
				promises.push(service.find(foreignKey).then(list => document[field.document] = list));
			}
		}

		return Promise.all(promises).then(() => document);
	}

	getDocuments(service, list, index) {
		if (list == null || list.length == 0) return Promise.resolve();
		if (index == null) index = 0;
		if (index >= list.length) return Promise.resolve();
		const item = list[index];
		console.log(`[${this.constructor.name}.getDocuments(${service.name}, ${index})] : updating references to register ${index}, item = ${JSON.stringify(item)}, list = `, list);
		return this.getDocument(service, item, false).then(() => this.getDocuments(service, list, ++index));
	}

	getPropertiesWithRef(serviceName, $ref) {
    	return this.openapi.get_properties_with_ref(this.path, this.method, this.schema_place, $ref);
	}
    // devolve o rufsService apontado por field
    getForeignService(service, fieldName, debug) {
    	let field = undefined;
    	// TODO : refatorar consumidores da função getForeignService(field), pois pode haver mais de uma referência
    	if (service.properties != undefined) {
    		field = service.properties[fieldName];
    	} else {
    		field = this.openapi.get_property(this.path, this.method, this.schema_place, fieldName);
    	}

    	if (field == undefined) {
    		console.error(`[${this.constructor.name}.getForeignService(${service.name}, ${fieldName})] : fail to find property`);
    		if (debug == undefined)
		    	this.getForeignService(service, fieldName, true);
    		return undefined;
    	}

        return this.getSchema(field.$ref);
    }
	// (service, (service.field|foreignTableName), service.obj) => [{name: constraintName, table: foreignTableName, foreignKey: {}}]
	getPrimaryKeyForeign(schema, property_name, obj) {
		//if (schema.name != undefined && this.openapi.get_schema(this.path, this.method, this.schema_place) != null) schema = schema.name;
		console.log(`[${this.constructor.name}.getPrimaryKeyForeign()] openapi.get_primary_key_foreign("${schema.name}", "${property_name}", json!(${JSON.stringify(obj)}));`);
		let ret = this.openapi.get_primary_key_foreign(schema.name, property_name, obj);
		//console.log(`[get_primary_key_foreign("${schema.name}", "${property_name}", ${JSON.stringify(obj)});] ret =`, ret);
		return ret;
	}

}
// differ to DataStore by instance and filter, aggregate, sort and pagination features
class DataStoreItem extends DataStore {

	constructor(dataStoreManager, path, method, schema_place, property_name, list) {
		super(dataStoreManager.openapi, path, method, schema_place, property_name, list);
		this.dataStoreManager = dataStoreManager;
		// instance
		this.instance = {};
		this.instanceFlags = {};
		// aggregate
		this.instanceAggregateRange = {};
		this.aggregateResults = new Map();
		// sort
		this.fieldsSort = {};
		// filter
		this.instanceFilter = {};
		this.instanceFilterRange = {};
		this.instanceFilterRangeMin = {};
		this.instanceFilterRangeMax = {};
		this.filterResults = this.list;
		// pagination
		this.pagination = new Pagination(100);
	}

	isValid() {
		const properties = this.properties || this.schema.properties;
		let ret = true;

		for (const [fieldName, property] of Object.entries(properties)) {
			if (property.essential == true && property.identityGeneration == null) {
				const value = this.instance[fieldName];

				if (value === undefined || (value === null && property.nullable != true)) {
					ret = false;
					console.log(`${this.constructor.name}.isValid() : invalid field content : ${fieldName} : value = ${value}, property :`, property);
					break;
				}
			}
		}

		return ret;
	}

	isChanged() {
		const properties = this.properties || this.schema.properties;
		let ret = false;

		for (const [fieldName, property] of Object.entries(properties)) {
			if (this.instance[fieldName] != this.original[fieldName]) {
				ret = true;
				break;
			}
		}

		return ret;
	}

	process(action, params) {
		return super.process(action, params).then(() => {
			return this.clearFilter().then(() => {
				this.clearSort();
				this.clearAggregate();
			});
		})
	}
// Instance section
	clear() {
		return super.clear().then(() => {
			this.instance = {};
			this.instanceFlags = {};
			return this.setValues({}, true, true); // set default values
		});
	}

	setValues(obj, enableDefault, enableNull) {
		console.log(`[${this.constructor.name}(${this.title}).setValues()]`, obj);

		const getDefaultValue = field => {
			let value;

			if (field.default != undefined) {
				if (field.type == "integer" && field.default != "") {
					value = Number.parseInt(field.default);
				} else if (field.type == "number" && field.default != "") {
					value = Number.parseFloat(field.default);
				} else if ((field.type.includes("date") || field.type.includes("time")) && field.default != "")  {
					value = new Date();
					value.setMilliseconds(0);
				} else {
					value = field.default;
				}
			} else {
				value = undefined;
			}

			return value;
		};

		// retorna um array de boolean, um elemento para cada bit, ou seja, cada caracter ascii hex gera quatro elementos.
		const strAsciiHexToFlags = (strAsciiHex, numBits) => {
			if (strAsciiHex == null || strAsciiHex.length == 0) {
				return null;
			}
	
			if (numBits == undefined) {
				numBits = 32;
			}
	
			const flags = new Array(numBits);
	
			for (let i = strAsciiHex.length-1, j = 0; i >= 0; i--) {
				let ch = strAsciiHex.charAt(i);
				let byte = parseInt(ch, 16);
	
				for (let k = 0; k < 4; k++, j++) {
					let bit = 1 << k;
					let value = byte & bit;
					let flag = value != 0 ? true : false;
		    		flags[j] = flag;
				}
			}
	
			return flags;
		}

		const setValue = (fieldName, obj) => {
			console.log(`[${this.constructor.name}(${this.title}).setValue(${fieldName})]`, obj);
			const field = this.properties[fieldName];
			delete field.externalReferencesStr;
			let value = obj[fieldName];

			if (value != undefined) {
				if (field.$ref != undefined) {
					if (field.type != "object" && (typeof(value) != "string" && typeof(value) != "number" && Array.isArray(value) == false)) {
						const foreignKey = this.dataStoreManager.openapi.get_foreign_key(this.name, fieldName, value);

						if (foreignKey != null) {
							obj[fieldName] = value = foreignKey.get(fieldName);
						}
					}

					field.externalReferencesStr = this.buildFieldStr(fieldName, obj);
				} else if (field.flags != null) {
					// field.flags : String[], vm.instanceFlags[fieldName] : Boolean[]
					this.instanceFlags[fieldName] = strAsciiHexToFlags(value.toString(16));
				} else if (field.enum != undefined) {
					let pos;

					if (value instanceof Object) {
						let strValue = JSON.stringify(value);
						pos = field.filterResultsStr.indexOf(strValue);
						field.externalReferencesStr = field.filterResultsStr[pos];
					} else {
						if (field.filterResults != undefined) {
							pos = field.filterResults.indexOf(value);
							field.externalReferencesStr = field.filterResultsStr[pos];
						} else {
//							debugger;
							console.error(`[${this.constructor.name}.setValues().setValue(${fieldName})] : field.filterResults is undefined !`);
//							console.trace();
						}
					}

					if (pos < 0) {
						console.error(`${this.constructor.name}.setValue(${fieldName}) : don\'t found\nvalue:`, value, `\nstr:\n`, field.externalReferences, `\noptions:\n`, field.filterResultsStr);
					}
				}
			}

			if (value != null && value instanceof Date && field.type == "string" && field.format == "date-time") {
				value = value.toISOString();
			}
			
			console.log(`[${this.constructor.name}.setValues().setValue()] openapi.copy_value("${this.path}", "${this.method}", SchemaPlace::${this.schema_place}, "${fieldName}", ${JSON.stringify(value)})`);
			let fieldValue;

			if (this.property_name == null) {
				fieldValue = this.openapi.copy_value(this.path, this.method, this.schema_place, fieldName, value);//value || {}
			} else if (value != null) {
				if (value instanceof Object) {
					fieldValue = value[fieldName];
				} else {
					fieldValue = value;
				}
			} else if (enableDefault) {
				fieldValue = getDefaultValue(field);
			}

			if (fieldValue != null && field.type == "string" && field.format == "date-time") {
				fieldValue = new Date(fieldValue);
			}
			
			console.log(`[${this.constructor.name}.setValues().setValue()] openapi.copy_value("${this.path}", "${this.method}", SchemaPlace::${this.schema_place}, "${fieldName}", ${JSON.stringify(value)}) :`, fieldValue, typeof fieldValue);
			this.instance[fieldName] = fieldValue;
			return Promise.resolve(value);
		}

		if (obj == undefined) {
			obj = {};
		}

		if (enableDefault == true) {
			for (let [fieldName, field] of Object.entries(this.properties)) {
				if (obj[fieldName] == undefined && field.default != undefined) {
					const value = getDefaultValue(field);
					if (value != undefined) obj[fieldName] = value;
				}
			}
		}

		if (enableNull == true) {
			for (let [fieldName, field] of Object.entries(this.properties)) {
				if (obj[fieldName] == undefined && field.nullable == true) {
					obj[fieldName] = null;
				}
			}
		}

		let promise;

		if (this.dataStoreManager.services[this.name] != undefined) {
			const service = this.dataStoreManager.getSchema(this.name);
			promise = this.dataStoreManager.getDocument(service, obj, false);
		} else {
			promise = this.dataStoreManager.getDocument(this, obj, false);
		}

		return promise.then(() => {
			const next = list => {
				if (list.length == 0) return obj;
				const fieldName = list.shift();
				return setValue(fieldName, obj).then(() => next(list));
			}

			const fieldNames = Object.keys(this.properties);
			console.log(`[${this.constructor.name}.setValues(${fieldNames})]`);
			return next(fieldNames);
		});
	}
// Aggregate Section
	clearAggregate() {
		this.instanceAggregateRange = {};
		this.aggregateResults = new Map();
	}
    // private
	buildField(stringBuffer, fieldName, obj) {
		//const value = this.openapi.get_value_from_schema(this.name, fieldName, obj)
		const value = obj[fieldName];

		if (value == null || value === "") {
			return stringBuffer;
		}

    	const field = this.properties[fieldName];

		if (field == undefined) {
			console.error("buildField : field ", fieldName, " don't found in properties, options are : ", this.properties);
			return stringBuffer;
		}

		if ((value instanceof Date) == false && (value instanceof Object) == true) {
			stringBuffer.push(JSON.stringify(value, null, "\t"));
			return stringBuffer;
		}

		const item = field.$ref != null ? this.dataStoreManager.getPrimaryKeyForeign(this, fieldName, obj) : null;

		if (item != null) {
			const service = this.dataStoreManager.getSchema(item.schema);

			if (service != undefined) {
				const primaryKey = item.primary_key;
				let pos = service.findPos(primaryKey);

				if (pos >= 0) {
					stringBuffer.push(service.listStr[pos]);
				} else {
					let pos = service.findPos(primaryKey);
//					console.error(`[${this.constructor.name}.buildField] don't find item from service ${service.name} with primaryKey ${JSON.stringify(primaryKey)}, used ${service.name}.getPrimaryKeyForeign(${JSON.stringify(obj)}, ${fieldName}, ${JSON.stringify(field.$ref)})`);
//					throw new Error(`this.buildField : don't find itemStr from service ${service.name}`);
				}
			} else {
				console.error(`[${this.constructor.name}.buildField] don't loaded service ${item.schema}`);
			}
		} else if (fieldName == "id") {
			// TODO : o "id" não deve fazer parte de StrValue, criar uma lista para armazenar os primaryKeys
			function padLeft(str, size, ch) {
				while (str.length < size) {
					str = ch + str;
				}

				return str;
			}

			stringBuffer.push(padLeft(value.toString(), 4, '0'));
		} else if (field.type.includes("date") || field.type.includes("time")) {
			stringBuffer.push(new Date(value).toLocaleString());
		} else {
			// TODO : verificar se o uso do "trim" não tem efeitos colaterais.
			stringBuffer.push(value.toString().trim());
		}

    	return stringBuffer;
    }
	// public
	buildFieldStr(fieldName, item) {
//		console.time("buildFieldStr" + "-" + fieldName);
		let stringBuffer = [];
		let str = "";
		this.buildField(stringBuffer, fieldName, item);
		if (stringBuffer.length > 0) str = stringBuffer.join(" - ");
//		console.timeEnd("buildFieldStr" + "-" + fieldName);
		return str;
	}

	applyAggregate(aggregate) {
		if (aggregate == undefined) aggregate = this.instanceAggregateRange; else this.instanceAggregateRange = aggregate;
		const dateRanges = ["secound", "minute", "hora", "dia", "mês", "ano"];
		
		const labelFromDate = (date, range) => {
			let type = dateRanges.indexOf(range);
			let str = "";
			if (type <= 5) str = date.getFullYear() + " " + str;
			if (type <= 4) str = date.getMonth()+1 + "/" + str;
			if (type <= 3) str = date.getDate() + "/" + str;
			if (type <= 2) str = date.getHours() + " " + str;
			return str;
		};
		
		this.aggregateResults = new Map();
		
		for (let item of this.filterResults) {
			let label = "";
			
			for (let fieldName in aggregate) {
				let value = item[fieldName];
				let range = aggregate[fieldName];
				let field = this.properties[fieldName];
				
				if (range != false && range != "" && range != 0) {
					if (field.$ref != undefined) {
						label = label + this.buildFieldStr(fieldName, item) + ",";
					} else if (field.flags != null) {
						label = label + value.toString(16) + ",";
					} else if (field.enum != undefined) {
						let pos = field.filterResults.indexOf(JSON.stringify(value));
						label = label + field.filterResultsStr[pos] + ",";
					} else if (field.htmlType == "number") {
						label = label + Math.trunc(value / range) * range + ",";
					} else if (field.htmlType.includes("date") || field.htmlType.includes("time")) {
						label = label + labelFromDate(value, range) + ",";
					}
				}
			}
			
			if (label.length > 0) {
				if (this.aggregateResults.has(label) == true) {
					this.aggregateResults.set(label, this.aggregateResults.get(label) + 1);
				} else {
					this.aggregateResults.set(label, 1);
				}
			}
		}
	}
// Sort section
	// format fieldsTable in correct order;
	orderFieldsSort() {
		const entries = Object.entries(this.fieldsSort);
		entries.sort((a, b) => a[1].orderIndex - b[1].orderIndex);
		this.fieldsTable = [];
		for (let [fieldName, field] of entries) if (field.hiden != true && field.tableVisible != false) this.fieldsTable.push(fieldName);
	}

	clearSort() {
		this.fieldsSort = {};
		//const properties = this.schemaResponse != undefined ? this.schemaResponse.properties : this.properties;

		for (let [fieldName, field] of Object.entries(this.properties)) {
			this.fieldsSort[fieldName] = {};
			this.fieldsSort[fieldName].type = field.type;
			this.fieldsSort[fieldName].orderIndex = field.orderIndex;
			this.fieldsSort[fieldName].sortType = field.sortType;
			this.fieldsSort[fieldName].tableVisible = field.tableVisible;
		}

		this.applySort();
	}
	// sortType, orderIndex, tableVisible
	applySort(sort) {
		if (sort != undefined) {
			for (let [fieldName, field] of Object.entries(this.fieldsSort)) {
				if (sort[fieldName] != undefined) {
					field.sortType = sort[fieldName].sortType;
					field.orderIndex = sort[fieldName].orderIndex;
					field.tableVisible = sort[fieldName].tableVisible;
				}
			}
		}
		// format fieldsTable in correct order;
		this.orderFieldsSort();

		this.filterResults.sort((a, b) => {
			let ret = 0;
			
			for (let fieldName of this.fieldsTable) {
				let field = this.fieldsSort[fieldName];
				
				if (field.sortType != undefined) {
					let valA = a[fieldName];
					let valB = b[fieldName];
					
					if (valA != valB) {
						if (valB == undefined) ret = -1;
						else if (valA == undefined) ret = +1;
						else if (field.type == "integer" || field.type == "number") ret = valA - valB;
						else if (field.type == "string") ret = valA.localeCompare(valB);
						else if (field.type == "boolean") ret = valA - valB;
						else if (field.type.includes("date") == true || field.type.includes("time") == true) ret = valA.valueOf() - valB.valueOf();
						if (field.sortType == "desc") ret *= -1;
						if (ret != 0) break;
					}
				}
			}
			
			return ret;
		});

		this.pagination.changePage();
	}

	sortToggle(fieldName) {
		const field = this.fieldsSort[fieldName];
		field.sortType = field.sortType == "asc" ? "desc" : "asc";
		this.applySort();
	}

	sortLeft(fieldName) {
		this.fieldsSort[fieldName].orderIndex--;
		this.applySort();
	}

	sortRigth(fieldName) {
		this.fieldsSort[fieldName].orderIndex++;
		this.applySort();
	}
// Filter section
	paginate(params) {
		if (params == undefined) params = {};
		//console.log(`${constructor.name}.paginate(${JSON.stringify(params)}) list = :`, filter, filterRangeMin, filterRangeMax);
		this.pagination.paginate(this.filterResults, params.pageSize, params.page);
		return Promise.resolve();
	}

	clearFilter() {
		// hora corrente, hora anterior, uma hora, hoje, ontem, um dia, semana corrente, semana anterior, uma semana, quinzena corrente, quinzena anterior, 15 dias, mês corrente, mês anterior, 30 dias, ano corrente, ano anterior, 365 dias
		this.instanceFilter = {};
		this.instanceFilterRange = {};
		this.instanceFilterRangeMin = {};
		this.instanceFilterRangeMax = {};
		this.filterResults = this.list;
		// TODO : verificar impacto
		return this.clear().then(() => {
			return this.paginate();
		});
	}

	applyFilter(filter, filterRangeMin, filterRangeMax) {
		if (filter == undefined) filter = this.instanceFilter; else this.instanceFilter = filter; 
		if (filterRangeMin == undefined) filterRangeMin = this.instanceFilterRangeMin; else this.instanceFilterRangeMin = filterRangeMin; 
		if (filterRangeMax == undefined) filterRangeMax = this.instanceFilterRangeMax; else this.instanceFilterRangeMax = filterRangeMax;
		console.log(`DataStoreItem.applyFilter() :`, filter, filterRangeMin, filterRangeMax);

		const compareExact = (a, b) => {
			if (typeof a == "string" && typeof b == "string")
				return a.trimEnd() == b.trimEnd();
			else
				return a == b;
		}

		const processForeign = (fieldFilter, obj, fieldName, compareType) => {
			const compareFunc = (candidate, expected, compareType) => {
				return Filter.matchObject(expected, candidate, (a,b,fieldName) => fieldName == undefined ? (compareType == 0 ? compareExact(a ,b) : (compareType < 0 ? a < b : a > b)) : false, false);
			}
			
			const item = this.dataStoreManager.getPrimaryKeyForeign(this.rufsService, fieldName, obj);
			const service = this.dataStoreManager.getSchema(item.schema);
			const primaryKey = item.primary_key;
			let candidate = service.findOne(primaryKey);
			let flag = compareFunc(candidate, fieldFilter.filter, 0);

			if (flag == true) {
				flag = compareFunc(candidate, fieldFilter.filterRangeMin, -1);

				if (flag == true) {
					flag = compareFunc(candidate, fieldFilter.filterRangeMax, 1);
				}
			}

			return flag;
		}

		const process = (expectedFields, expectedFieldsMin, expectedFieldsMax, list) => {
			const compareFunc = (candidate, expected, compareType) => {
				return Filter.matchObject(expected, candidate, (a,b,fieldName) => fieldName == undefined ? (compareType == 0 ? compareExact(a, b) : (compareType < 0 ? a < b : a > b)) : processForeign(a,candidate,fieldName, compareType), true);
			}
			
			return list.filter(candidate => {
				let flag = compareFunc(candidate, expectedFields, 0);

				if (flag == true) {
					flag = compareFunc(candidate, expectedFieldsMin, -1);

					if (flag == true) {
						flag = compareFunc(candidate, expectedFieldsMax, 1);
					}
				}

				return flag;
			});
		}

		const getFilteredItems = (objFilter, objFilterMin, objFilterMax) => {
			var list = [];

			if (objFilter != undefined && objFilter != null) {
				list = process(objFilter, objFilterMin, objFilterMax, this.list);
			} else {
				list = this.list;
			}

			return list;
		}
	
		this.filterResults = getFilteredItems(filter, filterRangeMin, filterRangeMax);
		console.log(`[${constructor.name}.applyFilter()] this.filterResults = `, this.filterResults);
		this.paginate();
	}

	setFilterRange(fieldName, range) {
		const periodLabels =     [" minuto ", " hora ", " dia ", " semana ", " quinzena ",    " mês ",     " ano "];
		const periods =          [        60,     3600,   86400,  7 * 86400,   15 * 86400, 30 * 86400, 365 * 86400];
		let period;
		
		for (let i = 0; i < periodLabels.length; i++) {
			if (range.includes(periodLabels[i])) {
				period = periods[i] * 1000;
				break;
			}
		}
		
		let now = new Date().valueOf();
		let nowPeriodTrunc = Math.trunc(now / period) * period; 
		let dateIni = undefined;
		let dateEnd = undefined;
		
		if (range.includes(" corrente ")) {
			dateIni = new Date(nowPeriodTrunc);
		} else if (range.includes(" anterior ")) {
			dateEnd = new Date(nowPeriodTrunc);
			dateIni = new Date(nowPeriodTrunc - period);
		} else {
			dateIni = new Date(now - period);
		}
		
		const nowDate = new Date(); 
		let dayActiveStart = dateFns.startOfDay(nowDate);
		let dayLastStart = dateFns.startOfDay(nowDate);
		dayLastStart.setDate(dayLastStart.getDate()-1);
		let weekActiveStart = dateFns.startOfWeek(nowDate);
		let weekLastStart = new Date(weekActiveStart);
		weekLastStart.setDate(weekLastStart.getDate()-7);
		let monthActiveStart = dateFns.startOfMonth(nowDate);
		let monthLastStart = new Date(monthActiveStart);
		monthLastStart.setMonth(monthLastStart.getMonth()-1);
		let yearActiveStart = dateFns.startOfYear(nowDate);
		let yearLastStart = new Date(yearActiveStart);
		yearLastStart.setFullYear(yearLastStart.getFullYear()-1);
		
		if (range.includes("dia corrente") == true) {
			dateIni = dayActiveStart;
		} else if (range.includes("dia anterior") == true) {
			dateIni = dayLastStart;
			dateEnd = dayActiveStart;
		} else if (range.includes("semana corrente") == true) {
			dateIni = weekActiveStart;
		} else if (range.includes("semana anterior") == true) {
			dateIni = weekLastStart;
			dateEnd = weekActiveStart;
		} else if (range.includes("quinzena corrente") == true) {
			dateIni = nowDate.getDate() <= 15 ? monthActiveStart : new Date(monthActiveStart.setDate(15));
		} else if (range.includes("quinzena anterior") == true) {
			dateEnd = nowDate.getDate() <= 15 ? monthActiveStart : new Date(monthActiveStart.setDate(15));
			dateIni = new Date(dateEnd);
			if (dateEnd.getDate() > 15) dateIni.setDate(15); else dateIni.setDate(1); 
		} else if (range.includes("mês corrente") == true) {
			dateIni = monthActiveStart;
		} else if (range.includes("mês anterior") == true) {
			dateIni = monthLastStart;
			dateEnd = monthActiveStart;
		} else if (range.includes("ano corrente") == true) {
			dateIni = yearActiveStart;
		} else if (range.includes("ano anterior") == true) {
			dateIni = yearLastStart;
			dateEnd = yearActiveStart;
		}
		
		this.instanceFilterRangeMin[fieldName] = dateIni;
		this.instanceFilterRangeMax[fieldName] = dateEnd;
	}

}
/*
IDBKeyRange.bound
IDBKeyRange.lowerBound
IDBKeyRange.upperBound
IDBKeyRange.only
 */
class Filter {
	// private
	static matchObject(expectedFields, actualObject, testFunc, matchStringPartial, recursive) {
        let flag = true;

        for (let key in expectedFields) {
            let expectedProperty = expectedFields[key];
            
            if (expectedFields.hasOwnProperty(key) && expectedProperty != undefined) {
                let actualProperty = actualObject[key];

                if (expectedProperty == null) {
                	if (actualProperty != null) {
                        flag = false;
                	}
                } else if (actualProperty == undefined || actualProperty == null) {
                    flag = false;
                } else if (expectedProperty instanceof Date) {
                	if (typeof actualProperty === "string") {
                		flag = testFunc(expectedProperty.valueOf(), Date.parse(actualProperty));
                	} else if (actualProperty instanceof Date) {
                		flag = testFunc(expectedProperty.valueOf(), actualProperty.valueOf());
                	} else {
                		flag = false;
                	}
                } else if (typeof expectedProperty === "number") {
                	if (typeof actualProperty === "number") {
                    	flag = testFunc(expectedProperty, actualProperty);
                	} else {
                		flag = false;
                	}
                } else if (typeof expectedProperty === "string") {
                	if (typeof actualProperty === "string") {
                    	if (matchStringPartial == true) {
                            if (expectedProperty != "") {
                                flag = (actualProperty.trimEnd().indexOf(expectedProperty.trimEnd()) >= 0);
                            }
                        } else {
                            flag = (actualProperty.trimEnd() == expectedProperty.trimEnd());
                        }
                	} else {
                		flag = false;
                	}
                } else if (expectedProperty instanceof Object) {
                	if (recursive == true) {
	                    flag = matchObject(expectedProperty, actualProperty, matchStringPartial, recursive, testFunc);
                	} else {
                    	flag = testFunc(expectedProperty, actualProperty, key);
                	}
                } else {
                	throw new Error(`Invalid type of field ${key}, contents : ${expectedProperty}`);
                }
                
                if (flag == false) {
                	return false;
                }
            }
        }

        return flag;
    }
	// public
	static checkMatchExact(item, obj) {
		if (obj == null) {
			return item == null
		}

		const itemEntries = Object.entries(item)
		const objEntries = Object.entries(obj)

		if (itemEntries.length == 0 && objEntries.length == 0) {
			return true
		} else if (itemEntries.length == 0 || objEntries.length == 0) {
			return false
		}

    	let match = true;

    	for (let [fieldName, expected] of objEntries) {
        	if (typeof expected == "string") expected = expected.trimEnd();
        	let value = item[fieldName];
        	if (typeof value == "string") value = value.trimEnd();

        	if (value != expected) {
        		match = false;
        		break;
        	}
    	}

    	return match;
	}
	// public
	static find(list, obj) {
		if (obj == null) return list;
		return list.filter(item => Filter.checkMatchExact(item, obj));
	}
	// public
	static findOne(list, obj, callback) {
		if (obj == null) return null;
		var ret = null;

        for (var i = 0; i < list.length; i++) {
        	var item = list[i];
        	var match = Filter.checkMatchExact(item, obj);

        	if (match == true) {
        		ret = item;

        		if (callback) {
        			callback(i, item);
        		}

        		break;
        	}
        }

        return ret;
	}
	// public
	static findPos(list, params) {
		var ret = -1;
        Filter.findOne(list, params, pos => ret = pos);
        return ret;
	}
	// public
	static findOneIn(list, listParams) {
		var filterResults = [];

		if (list.length > 0) {
			for (var params of listParams) {
				filterResults.push(Filter.findOne(list, params));
			}
		}

		return filterResults;
	}

}

class Pagination {

    constructor(pageSize, page) {
    	this.list = [];
    	this.setPageSize(pageSize);
    	this.setPage(page);
    }
    
    setPageSize(pageSize) {
    	return this.paginate(this.list, pageSize);
    }

    setPage(page) {
    	return this.paginate(this.list, this.pageSize, page);
    }

    paginate(list, pageSize, page) {
    	if (pageSize != undefined) this.pageSize = pageSize; else this.pageSize = 100;
    	if (page != undefined) this.currentPage = page; else this.currentPage = 1;
    	this.list = list;
        var result = Math.ceil(list.length/this.pageSize);
        this.numPages = (result == 0) ? 1 : result;
    	this.changePage();
    }

    changePage() {
     	this.listPage = this.list.slice((this.currentPage-1) * this.pageSize, this.currentPage * this.pageSize);
    }

}

export {RufsSchema, DataStore, DataStoreManager, DataStoreItem, Filter, Pagination}
