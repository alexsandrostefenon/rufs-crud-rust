class CaseConvert {

    static camelToUnderscore(str, checkLastIsUpper) {
		str = str.trim();
		var ret = "";
		var lastIsUpper = true;

		for (var i = 0; i < str.length; i++) {
			var ch = str[i];

			if (ch >= 'A' && ch <= 'Z') {
				ch = ch.toLowerCase();

				if (checkLastIsUpper != false && lastIsUpper == true) {
					ret = ret + ch;
				} else {
					ret = ret + '_' + ch;
				}

				lastIsUpper = true;
			} else {
				ret = ret + ch;
				lastIsUpper = false;
			}
		}

		if (ret.length > 0 && ret[0] == '_') {
			ret = ret.substring(1);
		}

		return ret;
    }

    static underscoreToCamel(str, isFirstUpper) {
		str = str.trim();
    	const regExp = /[a-zA-Z]/;
		var ret = "";
		var nextIsUpper = false;

		if (isFirstUpper == true) {
			nextIsUpper = true;
		}

		for (var i = 0; i < str.length; i++) {
			var ch = str[i];

			if (nextIsUpper == true) {
				ch = ch.toUpperCase();
				nextIsUpper = false;
			} else {
//				ch = ch.toLowerCase();
			}

			if (ch == '_' && str.length > i && regExp.test(str[i+1]) == true) {
				nextIsUpper = true;
			} else {
				ret = ret + ch;
			}
		}

		return ret;
    }

    static camelUpToCamelLower(str) {
		if (str == null) {
			return null;
		}

		str = str.trim();
		var ret = str;

		if (str.length > 0) {
			ret = str.charAt(0).toLocaleLowerCase() + str.substring(1);
		}

		return ret;
    }

	static caseAnyToLabel(str) {
		if (str == null) {
			return "";
		}

		str = str.trim();
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

		return ret;
	}

}

export {CaseConvert}
