import {CrudCommom} from "./CrudCommom.js";
import {RufsSchema} from "./DataStore.js";

class CrudItem extends CrudCommom {

	constructor(serverConnection, path, method, schema_place, property_name, primaryKeyForeign, title, numMaxItems, queryCallback, selectCallback) {
		console.log(`[CrudItem.constructor] : ${path}, ${property_name}, ${title}`, primaryKeyForeign);
    	super(serverConnection, path, method, schema_place, property_name);
		this.fieldName = property_name;
		const field = this.properties[property_name];
		this.title = (title != undefined && title != null) ? title : field.title;
		this.isClonable = field.isClonable == undefined ? false : field.isClonable;
		this.numMaxItems = (numMaxItems != undefined && numMaxItems != null) ? numMaxItems : 999;
		this.queryCallback = queryCallback;
		this.selectCallback = selectCallback;
		console.log(`[CrudItem.constructor()] openapi.get_foreign_key("${this.name}", "${this.fieldName}", &json!(${JSON.stringify(primaryKeyForeign)}))`);
		const foreignKey = this.serverConnection.openapi.get_foreign_key(this.name, this.fieldName, primaryKeyForeign);
		this.foreignKey = Object.fromEntries(foreignKey);
		
		for (let [_fieldName, value] of Object.entries(this.foreignKey)) {
			this.properties[_fieldName].hiden = true;
			this.properties[_fieldName].tableVisible = false;
			this.properties[_fieldName].default = value;
		}

		this.query();
	}

	query() {
		const params = {};

		if (this.foreignKey == null || Object.entries(this.foreignKey).length > 0) {
			params.filter = this.foreignKey;
		}
		
		return this.process("search", params).then(() => {
			if (this.queryCallback != undefined && this.queryCallback != null) {
				this.queryCallback(this.filterResults);
			}

			this.serverConnection.$scope.$apply();
		});
	}

	clone(primaryKeyForeign) {
		this.foreignKey = Object.fromEntries(this.serverConnection.openapi.get_foreign_key(this.name, this.fieldName, primaryKeyForeign));

		if (this.isClonable == true) {
			let count = 0;

			for (var item of this.filterResults) {
				let newItem = angular.copy(item);
				
				for (let fieldName in this.foreignKey) {
					this.newItem[fieldName] = this.foreignKey[fieldName];
				}
				
				this.rufsService.save(newItem).then(response => {
					count++;

					if (count == this.filterResults.length) {
						this.query();
					}
				});
			}
		} else {
			this.query();
		}
	}

    validateFieldChange(fieldName, newValue, oldValue) {
    	let ret = super.validateFieldChange(fieldName, newValue, oldValue);

    	if (ret == true && this.selectCallback != undefined) {
    		if (newValue == undefined) 
    			newValue = this.instance[fieldName];
    		else
    			this.instance[fieldName] = newValue;

    		this.selectCallback(fieldName);
			// update UI
			this.setValues(this.instance, false, false);
    	}

    	return ret;
    }

	remove(primaryKey) {
        // data may be null
		return super.remove(primaryKey).then(data => this.query());
	}

	save() {
		return super.save().then(response => this.query()).then(() => {
			for (let fieldName in this.properties) {
				let field = this.properties[fieldName];
				
				if (field.hiden != true) {
					document.getElementById(this.formId + "-" + fieldName).focus();
					break;
				}
			}

			this.serverConnection.$scope.$apply();
		});
	}

	update() {
		return super.update().then(response => this.query());
	}
}

export {CrudItem}
