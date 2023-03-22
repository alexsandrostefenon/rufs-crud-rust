import {CrudUiSkeleton} from "./CrudUiSkeleton.js";

class CrudObjJson extends CrudUiSkeleton {

	constructor(parent, properties, fieldNameExternal, title, serverConnection, selectCallback) {
		super(serverConnection, fieldNameExternal, {"properties": properties}, selectCallback);
		this.parent = parent;
		this.fieldNameExternal = fieldNameExternal;
		this.title = title || this.parent.properties[this.fieldNameExternal].title || this.serverConnection.convertCaseAnyToLabel(this.fieldNameExternal);

		for (var fieldName in this.properties) {
			var field = this.properties[fieldName];
			field._label = serverConnection.convertCaseAnyToLabel(fieldName);
		}
	}

	process(action, params) {
		return this.buildFieldFilterResults().then(() => super.process(action, params));
	}

	get(parentInstance) {
		return this.process(this.action).
		then(() => {
			const data = parentInstance[this.fieldNameExternal] || {};
			const obj = typeof(data) == "string" ? JSON.parse(data) : data;
			return this.setValues(obj, false, false);
		});
	}

	save() {
		if (this.fieldNameExternal != undefined && this.fieldNameExternal != null && this.fieldNameExternal.length > 0) {
			if (this.parent.properties[this.fieldNameExternal].type == "string")
				this.parent.instance[this.fieldNameExternal] = JSON.stringify(this.instance);
			else
				this.parent.instance[this.fieldNameExternal] = this.instance;
		}
	}

}

CrudUiSkeleton.CrudObjJson = CrudObjJson;

export {CrudObjJson}
