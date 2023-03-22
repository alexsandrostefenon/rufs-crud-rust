import {Filter} from "./DataStore.js";
import {Utils} from "./Utils.js";
import {CaseConvert} from "./CaseConvert.js";
import {CrudUiSkeleton} from "./CrudUiSkeleton.js";

class CrudItemJson extends CrudUiSkeleton {

	constructor(serverConnection, path, method, schema_place, property_name, selectCallback, parent, title, options) {
		/*
		let _fields = {};
		_fields._name = {};
		_fields._name.type = "string";
		if (options == null || options.disableOrderByName != true) _fields._name.sortType = "asc";

		if (options != null && options.nameOptions != undefined) {
			_fields._name.enum = options.nameOptions;
		}
		
		for (let fieldName in properties) _fields[fieldName] = properties[fieldName];
		*/
		super(serverConnection, path, method, schema_place, property_name, selectCallback);
		this.primaryKeys = ["_name"];
		this.parent = parent;
		this.fieldNameExternal = property_name;
		this.title = title || this.parent.properties[this.fieldNameExternal].title || this.serverConnection.convertCaseAnyToLabel(this.fieldNameExternal);
		if (options != null) this.nameOptions = options.nameOptions;
	}

	process(action, params) {
		return this.buildFieldFilterResults().then(() => super.process(action, params));
	}

	get(parentInstance) {
		const data = parentInstance[this.fieldNameExternal] || {};
		const obj = typeof(data) == "string" ? JSON.parse(data) : data;
		this.list = [];

		for (var itemName in obj) {
			var objItem = obj[itemName];
			var item = {};

			for (var fieldName in this.properties) {
				var field = this.properties[fieldName];
				item[fieldName] = objItem[fieldName];
			}

			item._name = itemName;
			this.list.push(item);
		}
		
		return this.restrictNameOptions().then(() => this.process(this.action));
	}

	restrictNameOptions() {
		let promise;

		if (this.nameOptions != undefined) {
			this.properties._name.enum = [];
			for (let name of this.nameOptions) if (this.list.find(item => item._name == name) == undefined) this.properties._name.enum.push(name);
			promise = this.buildFieldFilterResults();
		} else {
			promise = Promise.resolve();
		}

		return promise;
	}
	// private, use in addItem, updateItem and removeItem
	updateParent() {
		const clone = (objRef, fields) => {
			var obj = {};
			if (fields == undefined) fields = Object.keys(objRef);
	
			for (var fieldName of fields) {
				obj[fieldName] = objRef[fieldName];
			}
	
			return obj;
		}

		var objItems = {};

		for (let item of this.list) {
			let obj = clone(item, Object.keys(this.properties));
			delete obj._name;
			objItems[item._name] = obj;
		}

		this.parent.instance[this.fieldNameExternal] = objItems
		return this.restrictNameOptions().then(() => {
			this.parent.rufsService.params.saveAndExit = false;
			return this.parent.update().then(res => this.get(res.data));
		});
	}

	save() {
		this.instance._name =  CaseConvert.underscoreToCamel(this.instance._name);
		// já verifica se é um item novo ou um update
		var isNewItem = true;

		for (var i = 0; i < this.list.length; i++) {
			var item = this.list[i];

			if (item._name == this.instance._name) {
				this.list[i] = this.instance;
				isNewItem = false;
				break;
			}
		}

		if (isNewItem == true) {
			this.list.push(this.instance);
		}

		return this.updateParent();
	}

	remove(name) {
		const index = Filter.findPos(this.list, {"_name": name});
		this.list.splice(index, 1);
		this.updateParent();
	}

	edit(name) {
		return this.clear().then(() => {
			const index = Filter.findPos(this.list, {"_name": name});
			var item = this.list[index];
			let promise = Promise.resolve();

			if (this.nameOptions != undefined) {
				this.properties._name.enum.push(item._name);
				promise = this.buildFieldFilterResults();
			}

			return promise.then(() => this.setValues(item, false, false));
		});
	}

	moveUp(name) {
		const index = Filter.findPos(this.list, {"_name": name});

		if (index > 0) {
			var tmp = this.list[index-1];
			this.list[index-1] = this.list[index];
			this.list[index] = tmp;
		}

		this.updateParent();
	}

	moveDown(name) {
		const index = Filter.findPos(this.list, {"_name": name});

		if (index < (this.list.length-1)) {
			var tmp = this.list[index+1];
			this.list[index+1] = this.list[index];
			this.list[index] = tmp;
		}

		this.updateParent();
	}

}

CrudUiSkeleton.CrudItemJson = CrudItemJson;

export {CrudItemJson}
