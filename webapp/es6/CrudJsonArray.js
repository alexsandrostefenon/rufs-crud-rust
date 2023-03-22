import {Filter} from "./DataStore.js";
import {ServerConnection} from "./ServerConnection.js";
import {CrudUiSkeleton} from "./CrudUiSkeleton.js";

class CrudJsonArray extends CrudUiSkeleton {
	constructor(serverConnection, path, method, schema_place, property_name, selectCallback, parent, options) {
		const field = parent.properties[property_name];
		super(serverConnection, path, method, schema_place, property_name, selectCallback);
		this.parent = parent;
		this.fieldNameExternal = property_name;
		this.title = options.title || field.title || this.serverConnection.convertCaseAnyToLabel(this.fieldNameExternal);
		this.action = options.action || parent.action;
		this.convertString = field.type == "string";
		this.list = [];
		this.activeIndex = null;
	}

	get(parentInstance) {
		const data = parentInstance[this.fieldNameExternal];
		this.list = [];

		if (data != undefined) {
			if (Array.isArray(data)) {
				this.list = data;
			} else if ((typeof data === 'string' || data instanceof String) && data.length > 0) {
				this.list = JSON.parse(data);
			}

			console.log(`[${this.constructor.name}.get()] this.list = `, this.list);
		}
		
		return this.process(this.action);
	}
	// private, use in addItem, updateItem and removeItem
	updateParent() {
		if (this.convertString == true) {
			this.parent.instance[this.fieldNameExternal] = JSON.stringify(this.list);
		} else {
			this.parent.instance[this.fieldNameExternal] = this.list;
		}

		if (this.action != "edit") return this.paginate();
		return this.parent.update().
		then(() => this.clear()).
		then(() => this.setPage()).
		then(() => this.serverConnection.$scope.$apply())
	}

	save() {
		if (this.activeIndex != null) {
			this.list[this.activeIndex] = this.instance;
		} else {
			this.list.push(this.instance);
		}

		return this.updateParent();
	}

	remove(index) {
		this.list.splice(index, 1);
		return this.updateParent();
	}

	edit(index) {
		this.activeIndex = index;
		
		this.clear().then(() => {
			var item = this.list[index];
			return this.setValues(item, false, false);
		});
	}

	moveUp(index) {
		if (index > 0) {
			var tmp = this.list[index-1];
			this.list[index-1] = this.list[index];
			this.list[index] = tmp;
		}

		return this.updateParent();
	}

	moveDown(index) {
		if (index < (this.list.length-1)) {
			var tmp = this.list[index+1];
			this.list[index+1] = this.list[index];
			this.list[index] = tmp;
		}

		return this.updateParent();
	}

	buildFieldStr(fieldName, item) {
		let ret = super.buildFieldStr(fieldName, item);

		if (ret == "") {
			ret = super.buildFieldStr(fieldName, item);
		}

		return ret;
	}

}

CrudUiSkeleton.CrudJsonArray = CrudJsonArray;

export {CrudJsonArray}
